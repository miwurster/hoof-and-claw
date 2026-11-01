use std::sync::{Arc, Mutex, PoisonError};

use guessing::PlayerId;
use prices::LatestPrice;

use super::{Broadcast, Fanout, PlayerEvent, Subscription};

/// A `Fanout` for tests: delivers like `Broadcast`, and records every published price and Player event in order.
#[derive(Clone, Default)]
pub struct Recording {
    delivery: Broadcast,
    published: Arc<Mutex<Vec<LatestPrice>>>,
    notified: Arc<Mutex<Vec<(PlayerId, PlayerEvent)>>>,
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

    /// Every Player event published so far, with its Player, oldest first.
    #[must_use]
    pub fn notified(&self) -> Vec<(PlayerId, PlayerEvent)> {
        self.notified
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

    fn notify_player(&self, player: PlayerId, event: PlayerEvent) {
        self.notified
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((player, event));
        self.delivery.notify_player(player, event);
    }

    fn subscribe(&self, player: PlayerId) -> Subscription {
        self.delivery.subscribe(player)
    }

    fn close(&self) {
        self.delivery.close();
    }
}
