use std::sync::Arc;

use prices::LatestPrice;
use tokio::sync::watch;

use super::{Delivery, Fanout, Subscription};

/// The in-process `Fanout`: one `watch` slot for the price that every stream reads.
#[derive(Clone)]
pub struct Broadcast {
    price: Arc<watch::Sender<Delivery>>,
}

impl Broadcast {
    #[must_use]
    pub fn new() -> Self {
        let (price, _) = watch::channel(Delivery::Price(LatestPrice::default()));
        Self {
            price: Arc::new(price),
        }
    }
}

impl Default for Broadcast {
    fn default() -> Self {
        Self::new()
    }
}

impl Fanout for Broadcast {
    fn publish_price(&self, latest: LatestPrice) {
        self.price.send_if_modified(|delivery| {
            if *delivery == Delivery::Closed {
                return false;
            }
            *delivery = Delivery::Price(latest);
            true
        });
    }

    fn subscribe(&self) -> Subscription {
        Subscription::new(self.price.subscribe())
    }

    fn close(&self) {
        self.price.send_replace(Delivery::Closed);
    }
}
