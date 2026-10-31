//! The `Fanout` port: hands the latest price to every open stream (ADR 0010).

mod broadcast;
mod recording;
#[expect(
    clippy::missing_panics_doc,
    reason = "a test suite fails by panicking, as every test does"
)]
pub mod suite;

use prices::LatestPrice;
use tokio::sync::watch;

pub use broadcast::Broadcast;
pub use recording::Recording;

/// Publishes events to the open streams.
pub trait Fanout: Clone + Send + Sync + 'static {
    /// Sends the latest price, or a change between live and Price delayed, to every open stream.
    fn publish_price(&self, latest: LatestPrice);

    /// Opens the events of one stream, starting after what has been published so far.
    fn subscribe(&self) -> Subscription;

    /// Ends every open stream, and every stream opened afterwards.
    fn close(&self);
}

/// What one stream receives: a slot that holds only the newest price, so a slow reader skips older ones (ADR 0007).
pub struct Subscription {
    price: watch::Receiver<Delivery>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Delivery {
    Price(LatestPrice),
    Closed,
}

impl Subscription {
    fn new(price: watch::Receiver<Delivery>) -> Self {
        Self { price }
    }

    /// Waits for the next price, or `None` once the stream has been closed.
    pub async fn next(&mut self) -> Option<LatestPrice> {
        if *self.price.borrow() == Delivery::Closed {
            return None;
        }
        self.price.changed().await.ok()?;
        match *self.price.borrow_and_update() {
            Delivery::Price(latest) => Some(latest),
            Delivery::Closed => None,
        }
    }
}
