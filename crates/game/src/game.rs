use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use jiff::Timestamp;
use prices::{LatestPrice, PriceSource};
use tokio::time::Instant;

use crate::{Clock, Fanout, Subscription};

/// The server publishes at most one price a second for all streams together (ADR 0010).
const PRICE_INTERVAL: Duration = Duration::from_secs(1);

/// The running game, shared by the task that takes in prices and by every request.
#[derive(Clone)]
pub struct Game<C, F> {
    clock: C,
    fanout: F,
    prices: Arc<Mutex<Prices>>,
}

/// What a stream shows when it opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub server_time: Timestamp,
    pub latest: LatestPrice,
}

#[derive(Default)]
struct Prices {
    /// What the Price source reported last.
    latest: LatestPrice,
    /// What the streams were sent last.
    published: LatestPrice,
}

impl<C: Clock, F: Fanout> Game<C, F> {
    pub fn new(clock: C, fanout: F) -> Self {
        Self {
            clock,
            fanout,
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

    /// The snapshot a stream opens with; subscribe first, so nothing published in between is lost (ADR 0010).
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            server_time: self.clock.now(),
            latest: self.lock().latest,
        }
    }

    pub fn subscribe(&self) -> Subscription {
        self.fanout.subscribe()
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
