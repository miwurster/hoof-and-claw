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

/// How many Player events a stream may hold unread before the adapter closes it (ADR 0010).
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
    ///
    /// A stream of a visitor who is no Player yet gets the prices only.
    fn subscribe(&self, player: Option<PlayerId>) -> Subscription;

    /// Ends every open stream, and every stream opened afterwards.
    fn close(&self);
}

/// What one stream receives: a slot that holds only the newest price, so a slow reader skips older ones (ADR 0007),
/// and a bounded queue of its Player's events, none of which is skipped.
pub struct Subscription {
    price: watch::Receiver<Delivery>,
    player: Option<mpsc::Receiver<PlayerEvent>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Delivery {
    Price(LatestPrice),
    Closed,
}

impl Subscription {
    fn new(price: watch::Receiver<Delivery>, player: Option<mpsc::Receiver<PlayerEvent>>) -> Self {
        Self { price, player }
    }

    /// Waits for the next event, or `None` once the stream has been closed.
    ///
    /// A Player event that is waiting goes before the price.
    pub async fn next(&mut self) -> Option<Event> {
        if *self.price.borrow() == Delivery::Closed {
            return None;
        }
        let price = match &mut self.player {
            Some(player) => tokio::select! {
                biased;
                event = player.recv() => return event.map(Event::Player),
                changed = self.price.changed() => changed,
            },
            None => self.price.changed().await,
        };
        price.ok()?;
        match *self.price.borrow_and_update() {
            Delivery::Price(latest) => Some(Event::Price(latest)),
            Delivery::Closed => None,
        }
    }
}
