use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use guessing::{Direction, GuessId, OpenGuess, PlayerId, Refusal, Score, StoreError};
use identity::Token;
use jiff::Timestamp;
use prices::{LatestPrice, PriceSource, ReceivedPrice, Report, Source};
use tokio::time::Instant;

use crate::{Clock, Fanout, PlayerEvent, Subscription};

/// The server publishes at most one price a second for all streams together (ADR 0010).
const PRICE_INTERVAL: Duration = Duration::from_secs(1);

/// Every store port the game writes to; `sqlite::Database` is one, and a test can join the in-memory fakes into one.
pub trait Store: guessing::Store + identity::Store {}

impl<S: guessing::Store + identity::Store> Store for S {}

/// The running game, shared by the task that takes in prices and by every request.
#[derive(Clone)]
pub struct Game<C, F, S> {
    clock: C,
    fanout: F,
    store: S,
    prices: Arc<Mutex<Prices>>,
}

/// An Open guess with the time it has left by the server's clock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Open {
    pub guess: OpenGuess,
    pub remaining: Duration,
}

/// What a stream shows when it opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub server_time: Timestamp,
    pub latest: LatestPrice,
    pub score: Score,
    pub open_guess: Option<Open>,
}

/// The store could not be read or written, or no random bytes were available for a new ID.
#[derive(Debug, thiserror::Error)]
#[error("the game cannot reach its store: {0}")]
pub struct Unavailable(#[source] Box<dyn std::error::Error + Send + Sync>);

#[derive(Debug, thiserror::Error)]
pub enum PlaceError {
    #[error(transparent)]
    Refused(Refusal),
    #[error(transparent)]
    Unavailable(#[from] Unavailable),
}

impl From<identity::StoreError> for Unavailable {
    fn from(error: identity::StoreError) -> Self {
        Self(Box::new(error))
    }
}

impl From<StoreError> for PlaceError {
    fn from(error: StoreError) -> Self {
        match error {
            StoreError::OpenGuessExists => Self::Refused(Refusal::OpenGuessExists),
            StoreError::Unavailable(error) => Self::Unavailable(Unavailable(error)),
        }
    }
}

#[derive(Default)]
struct Prices {
    /// What the Price source reported last.
    latest: LatestPrice,
    /// The last price received, with when the server received it: the Start price of a Guess placed now.
    last_received: Option<ReceivedPrice>,
    /// What the streams were sent last.
    published: LatestPrice,
}

/// 10 random bytes for a new ID.
fn random() -> Result<[u8; 10], Unavailable> {
    let mut bytes = [0; 10];
    getrandom::fill(&mut bytes).map_err(|error| Unavailable(Box::new(error)))?;
    Ok(bytes)
}

impl<C: Clock, F: Fanout, S: Store> Game<C, F, S> {
    pub fn new(clock: C, fanout: F, store: S) -> Self {
        Self {
            clock,
            fanout,
            store,
            prices: Arc::default(),
        }
    }

    /// Takes in every report of the Price source until it ends, and publishes the price as it changes.
    ///
    /// A changed price goes out at once after a quiet second; a later change within the same second goes out when the
    /// second ends, with the newest price. A change between live and Price delayed goes out at once.
    /// The throttle runs on `tokio`'s real time, not on the `Clock` (ADR 0011).
    pub async fn run(&self, mut source: impl PriceSource) {
        let mut last_price_sent: Option<Instant> = None;
        let mut pending = false;
        loop {
            let due = last_price_sent.map_or_else(Instant::now, |sent| sent + PRICE_INTERVAL);
            tokio::select! {
                report = source.next() => {
                    let Some(report) = report else { return };
                    let mut prices = self.lock();
                    prices.latest.apply(report);
                    if let Report::Received(price) = report {
                        // The controllable source of the test controls stands in for Bitstamp.
                        prices.last_received = Some(ReceivedPrice {
                            price,
                            source: Source::Bitstamp,
                            received_at: self.clock.now(),
                        });
                    }
                    if prices.latest.is_delayed() != prices.published.is_delayed() {
                        self.publish(&mut prices);
                    } else if prices.latest.price() != prices.published.price() {
                        if Instant::now() >= due {
                            self.publish(&mut prices);
                            last_price_sent = Some(Instant::now());
                        } else {
                            pending = true;
                        }
                    }
                }
                () = tokio::time::sleep_until(due), if pending => {
                    pending = false;
                    let mut prices = self.lock();
                    if prices.latest != prices.published {
                        self.publish(&mut prices);
                        last_price_sent = Some(Instant::now());
                    }
                }
            }
        }
    }

    /// Places a Guess for the browser's token: links the token to a Player on its first Guess, saves the Guess, and
    /// publishes Guess placed to the Player's streams before it answers (ADR 0010).
    ///
    /// # Errors
    ///
    /// Refuses with `PriceDelayed` while no live price is there, and with `OpenGuessExists` while the Player has an Open
    /// guess; fails when the store is unavailable.
    pub async fn place_guess(
        &self,
        token: &Token,
        direction: Direction,
    ) -> Result<Open, PlaceError> {
        let (last_received, delayed) = {
            let prices = self.lock();
            (prices.last_received, prices.latest.is_delayed())
        };
        let now = self.clock.now();
        // The price is checked before anything is written, so a refused Guess stores no Player.
        let candidate = OpenGuess::place(
            GuessId::new(now, random()?),
            PlayerId::new(now, random()?),
            direction,
            last_received,
            delayed,
            now,
        )
        .map_err(PlaceError::Refused)?;
        let player = identity::Store::link(&self.store, token.hash(), candidate.player)
            .await
            .map_err(Unavailable::from)?;
        let guess = OpenGuess {
            player,
            ..candidate
        };
        guessing::Store::place(&self.store, &guess).await?;
        let open = Open {
            guess,
            remaining: guess.remaining(now),
        };
        self.fanout
            .notify_player(player, PlayerEvent::GuessPlaced(open));
        Ok(open)
    }

    /// The Player the browser's token belongs to, or `None` before its first Guess.
    ///
    /// # Errors
    ///
    /// Fails when the store is unavailable.
    pub async fn player(&self, token: &Token) -> Result<Option<PlayerId>, Unavailable> {
        Ok(identity::Store::player(&self.store, token.hash()).await?)
    }

    /// The snapshot a stream opens with; subscribe first, so nothing published in between is lost (ADR 0010).
    ///
    /// A visitor who is no Player yet reads as Score 0 with no Open guess, and nothing is written (ADR 0001).
    ///
    /// # Errors
    ///
    /// Fails when the store is unavailable.
    pub async fn snapshot(&self, player: Option<PlayerId>) -> Result<Snapshot, Unavailable> {
        let standing = match player {
            Some(player) => guessing::Store::standing(&self.store, player)
                .await
                .map_err(|error| Unavailable(Box::new(error)))?,
            None => guessing::Standing::default(),
        };
        let server_time = self.clock.now();
        Ok(Snapshot {
            server_time,
            latest: self.lock().latest,
            score: standing.score,
            open_guess: standing.open_guess.map(|guess| Open {
                guess,
                remaining: guess.remaining(server_time),
            }),
        })
    }

    /// Opens the events of one stream: every price, and the Player's own events.
    pub fn subscribe(&self, player: Option<PlayerId>) -> Subscription {
        self.fanout.subscribe(player)
    }

    /// Ends every open stream, so the server can shut down.
    pub fn end_streams(&self) {
        self.fanout.close();
    }

    /// Puts the price back into its start state, Price delayed with no price, as the test controls' reset does.
    #[cfg(feature = "test-controls")]
    pub fn forget_the_price(&self) {
        let mut prices = self.lock();
        prices.latest = LatestPrice::default();
        prices.last_received = None;
        self.publish(&mut prices);
    }

    fn publish(&self, prices: &mut Prices) {
        prices.published = prices.latest;
        self.fanout.publish_price(prices.latest);
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Prices> {
        self.prices.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
