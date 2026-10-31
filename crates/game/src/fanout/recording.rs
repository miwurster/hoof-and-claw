use std::sync::{Arc, Mutex, PoisonError};

use prices::LatestPrice;

use super::{Broadcast, Fanout, Subscription};

/// A `Fanout` for tests: delivers like `Broadcast`, and records every published price in order.
#[derive(Clone, Default)]
pub struct Recording {
    delivery: Broadcast,
    published: Arc<Mutex<Vec<LatestPrice>>>,
}

impl Recording {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Every price published so far, oldest first.
    #[must_use]
    pub fn published(&self) -> Vec<LatestPrice> {
        self.published
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl Fanout for Recording {
    fn publish_price(&self, latest: LatestPrice) {
        self.published
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(latest);
        self.delivery.publish_price(latest);
    }

    fn subscribe(&self) -> Subscription {
        self.delivery.subscribe()
    }

    fn close(&self) {
        self.delivery.close();
    }
}
