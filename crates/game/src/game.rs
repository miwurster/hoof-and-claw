use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use guessing::{
    Direction, DueGuesses, GuessId, OpenGuess, PlayerId, Refusal, ResolvedGuess, Score, StoreError,
};
use identity::{Token, TokenHash};
use jiff::Timestamp;
use prices::{LatestPrice, PriceSource, ReceivedPrice, Report, Source};
use tokio::sync::watch;
use tokio::time::Instant;

use crate::{Clock, Fanout, PlayerEvent, Subscription};

/// The server publishes at most one price a second for all streams together (ADR 0010).
const PRICE_INTERVAL: Duration = Duration::from_secs(1);

/// The first wait before a failed save of resolutions is tried again; each further wait doubles, up to the longest.
const FIRST_RETRY: Duration = Duration::from_millis(100);
const LONGEST_RETRY: Duration = Duration::from_secs(5);

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
    /// The Open guesses, by when they fall due.
    due: Arc<Mutex<DueGuesses>>,
    /// The Player ID that the first Guess of a visitor with an open stream will take, so that stream gets the
    /// Player's events from that Guess on.
    visitors: Arc<Mutex<HashMap<TokenHash, PlayerId>>>,
    /// Whether the server is shutting down, which stops `run` at its next report.
    shutting_down: Arc<watch::Sender<bool>>,
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
    /// The Player's most recent Resolved guess, which the page shows until the next Guess.
    pub last_resolved: Option<ResolvedGuess>,
}

/// Why the game cannot answer.
#[derive(Debug, thiserror::Error)]
pub enum Unavailable {
    /// The store could not be read or written.
    #[error("the game cannot reach its store: {0}")]
    Store(#[source] Box<dyn std::error::Error + Send + Sync>),
    /// No random bytes were available for a new ID.
    #[error("the game has no random bytes for a new ID: {0}")]
    NoRandomBytes(#[source] getrandom::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum PlaceError {
    #[error(transparent)]
    Refused(Refusal),
    #[error(transparent)]
    Unavailable(#[from] Unavailable),
}

impl From<identity::StoreError> for Unavailable {
    fn from(error: identity::StoreError) -> Self {
        Self::Store(Box::new(error))
    }
}

impl From<StoreError> for PlaceError {
    fn from(error: StoreError) -> Self {
        match error {
            StoreError::OpenGuessExists => Self::Refused(Refusal::OpenGuessExists),
            StoreError::Unavailable(error) => Self::Unavailable(Unavailable::Store(error)),
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
    getrandom::fill(&mut bytes).map_err(Unavailable::NoRandomBytes)?;
    Ok(bytes)
}

impl<C: Clock, F: Fanout, S: Store> Game<C, F, S> {
    pub fn new(clock: C, fanout: F, store: S) -> Self {
        Self {
            clock,
            fanout,
            store,
            prices: Arc::default(),
            due: Arc::default(),
            visitors: Arc::default(),
            shutting_down: Arc::new(watch::Sender::new(false)),
        }
    }

    /// Puts every Open guess in the store into the index of due guesses, so a Guess placed before a restart resolves
    /// on the first changed price received after it; run once at start-up, before the first price, and answer how
    /// many there are.
    ///
    /// # Errors
    ///
    /// Fails when the store is unavailable.
    pub async fn rebuild_due_guesses(&self) -> Result<usize, Unavailable> {
        let open = guessing::Store::open_guesses(&self.store)
            .await
            .map_err(|error| Unavailable::Store(Box::new(error)))?;
        let count = open.len();
        let mut due = self.due_guesses();
        for guess in open {
            due.add(guess);
        }
        Ok(count)
    }

    /// Takes in every report of the Price source until it ends, and publishes the price as it changes.
    ///
    /// A changed price goes out at once after a quiet second; a later change within the same second goes out when the
    /// second ends, with the newest price. A change between live and Price delayed goes out at once.
    /// The throttle runs on `tokio`'s real time, not on the `Clock` (ADR 0011).
    ///
    /// Returns once the game shuts down, after the save of any resolution it is running.
    pub async fn run(&self, mut source: impl PriceSource) {
        let mut last_price_sent: Option<Instant> = None;
        let mut pending = false;
        let mut shutting_down = self.shutting_down.subscribe();
        loop {
            let due = last_price_sent.map_or_else(Instant::now, |sent| sent + PRICE_INTERVAL);
            tokio::select! {
                biased;
                _ = async { shutting_down.wait_for(|shutting_down| *shutting_down).await.is_ok() } => return,
                report = source.next() => {
                    let Some(report) = report else { return };
                    if let Report::Received(price, trade_id) = report {
                        // The controllable source of the test controls stands in for Bitstamp.
                        let received = ReceivedPrice {
                            price,
                            source: Source::Bitstamp,
                            trade_id,
                            received_at: self.clock.now(),
                        };
                        self.lock().last_received = Some(received);
                        self.resolve(received).await;
                    }
                    let mut prices = self.lock();
                    prices.latest.apply(report);
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
        let fresh = PlayerId::new(now, random()?);
        // A stream opened meanwhile subscribes as the same Player.
        let player = *self.waiting_visitors().entry(token.hash()).or_insert(fresh);
        // The price is checked before anything is written, so a refused Guess stores no Player.
        let candidate = OpenGuess::place(
            GuessId::new(now, random()?),
            player,
            direction,
            last_received,
            delayed,
            now,
        )
        .map_err(PlaceError::Refused)?;
        let player = identity::Store::link(&self.store, token.hash(), candidate.player)
            .await
            .map_err(Unavailable::from)?;
        // From now on the token belongs to a Player, whom every stream opened later subscribes as.
        self.waiting_visitors().remove(&token.hash());
        let guess = OpenGuess {
            player,
            ..candidate
        };
        guessing::Store::place(&self.store, &guess).await?;
        self.due_guesses().add(guess);
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
                .map_err(|error| Unavailable::Store(Box::new(error)))?,
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
            last_resolved: standing.last_resolved,
        })
    }

    /// Opens the events of one stream for the browser's token, and answers whose stream it is: the Player, or `None`
    /// for a visitor who has placed no Guess yet.
    ///
    /// A visitor's stream subscribes as the Player that the visitor's first Guess will create, so it gets that
    /// Player's events from the first Guess on.
    ///
    /// # Errors
    ///
    /// Fails when the store is unavailable, or no random bytes are available for the Player ID.
    pub async fn open_stream(
        &self,
        token: &Token,
    ) -> Result<(Option<PlayerId>, Subscription), Unavailable> {
        if let Some(player) = self.player(token).await? {
            return Ok((Some(player), self.fanout.subscribe(player)));
        }
        let fresh = PlayerId::new(self.clock.now(), random()?);
        let waiting = *self.waiting_visitors().entry(token.hash()).or_insert(fresh);
        // A first Guess may have linked the token since it was read, and taken its waiting Player ID out already.
        if let Some(player) = self.player(token).await? {
            let mut visitors = self.waiting_visitors();
            if visitors.get(&token.hash()) == Some(&waiting) {
                visitors.remove(&token.hash());
            }
            drop(visitors);
            return Ok((Some(player), self.fanout.subscribe(player)));
        }
        Ok((None, self.fanout.subscribe(waiting)))
    }

    /// Starts shutting down: publishes Price delayed, which refuses every Guess from now on with `PriceDelayed`, and
    /// stops `run` once the save of any resolution it is running is done (ADR 0010).
    pub fn shut_down(&self) {
        self.shutting_down.send_replace(true);
        let mut prices = self.lock();
        prices.latest.apply(Report::Delayed);
        self.publish(&mut prices);
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

    /// Resolves every Open guess the received price resolves, saves the resolutions, and only then publishes them
    /// (ADR 0010).
    ///
    /// The resolutions wait here with their End price until the save succeeds, so no later price can change it; a
    /// failed save is tried again, after a growing wait on `tokio`'s real time.
    async fn resolve(&self, received: ReceivedPrice) {
        let resolved = self.due_guesses().resolve(received);
        if resolved.is_empty() {
            return;
        }
        let mut wait = FIRST_RETRY;
        let saved = loop {
            match guessing::Store::resolve(&self.store, &resolved).await {
                Ok(saved) => break saved,
                Err(error) => {
                    tracing::error!(
                        guesses = resolved.len(),
                        retry_ms = wait.as_millis(),
                        "cannot save resolutions: {error}"
                    );
                    tokio::time::sleep(wait).await;
                    wait = (wait * 2).min(LONGEST_RETRY);
                }
            }
        };
        for resolution in saved {
            self.fanout.notify_player(
                resolution.guess.guess.player,
                PlayerEvent::GuessResolved(resolution),
            );
        }
    }

    fn waiting_visitors(&self) -> std::sync::MutexGuard<'_, HashMap<TokenHash, PlayerId>> {
        self.visitors.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn due_guesses(&self) -> std::sync::MutexGuard<'_, DueGuesses> {
        self.due.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn publish(&self, prices: &mut Prices) {
        prices.published = prices.latest;
        self.fanout.publish_price(prices.latest);
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Prices> {
        self.prices.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
