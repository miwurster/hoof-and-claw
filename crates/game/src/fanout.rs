//! The `Fanout` port: hands the latest price to every open stream, and a Player's events to that Player's streams
//! (ADR 0010).

mod broadcast;
mod recording;
#[expect(
    clippy::missing_panics_doc,
    clippy::unwrap_used,
    reason = "a test suite fails by panicking, as every test does"
)]
pub mod suite;

use guessing::{PlayerId, Resolution};
use prices::LatestPrice;
use tokio::sync::{mpsc, watch};

pub use broadcast::Broadcast;
pub use recording::Recording;

use crate::Open;

/// How many Player events a stream may hold unread before the adapter closes it, unless configured otherwise (ADR 0010).
pub const PLAYER_EVENT_QUEUE: usize = 16;

/// An event about one Player, for every open stream of that Player.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerEvent {
    GuessPlaced(Open),
    /// The Player's Guess resolved, with their Score after it.
    GuessResolved(Resolution),
}

/// What one stream receives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Price(LatestPrice),
    Player(PlayerEvent),
}

/// Publishes events to the open streams.
pub trait Fanout: Clone + Send + Sync + 'static {
    /// Sends the latest price, or a change between live and Price delayed, to every open stream.
    fn publish_price(&self, latest: LatestPrice);

    /// Sends an event to every open stream of one Player; a stream whose queue is full is closed instead.
    fn notify_player(&self, player: PlayerId, event: PlayerEvent);

    /// Opens the events of one stream, starting after what has been published so far.
    fn subscribe(&self, player: PlayerId) -> Subscription;

    /// Ends every open stream, and every stream opened afterwards.
    fn close(&self);
}

/// What one stream receives: a slot that holds only the newest price, so a slow reader skips older ones (ADR 0007),
/// and a bounded queue of its Player's events, none of which is skipped.
pub struct Subscription {
    price: watch::Receiver<Delivery>,
    player: mpsc::Receiver<PlayerEvent>,
    /// The price this stream has shown: the one it subscribed at, or the last one it was given.
    seen: LatestPrice,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Delivery {
    Price(LatestPrice),
    /// Closed, with the last price published before, which a stream that has not shown it gets before it ends.
    Closed(LatestPrice),
}

impl Delivery {
    const fn latest(self) -> LatestPrice {
        match self {
            Self::Price(latest) | Self::Closed(latest) => latest,
        }
    }
}

impl Subscription {
    fn new(price: watch::Receiver<Delivery>, player: mpsc::Receiver<PlayerEvent>) -> Self {
        let seen = price.borrow().latest();
        Self {
            price,
            player,
            seen,
        }
    }

    /// Waits for the next event, or `None` once the stream has been closed.
    ///
    /// A Player event that is waiting goes before the price. A stream closed before it showed the last price, such as
    /// Price delayed on shutdown, gets that price before it ends.
    pub async fn next(&mut self) -> Option<Event> {
        loop {
            if let Delivery::Closed(last) = *self.price.borrow() {
                if last == self.seen {
                    return None;
                }
                self.seen = last;
                return Some(Event::Price(last));
            }
            let price = tokio::select! {
                biased;
                event = self.player.recv() => match event {
                    Some(event) => return Some(Event::Player(event)),
                    // A closed queue ends the stream, unless the whole fanout closed and the last price is due.
                    None if matches!(*self.price.borrow(), Delivery::Closed(_)) => continue,
                    None => return None,
                },
                changed = self.price.changed() => changed,
            };
            price.ok()?;
            // A closed slot is read at the top of the loop.
            if let Delivery::Price(latest) = *self.price.borrow_and_update() {
                self.seen = latest;
                return Some(Event::Price(latest));
            }
        }
    }
}
