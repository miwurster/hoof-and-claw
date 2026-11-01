use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use guessing::PlayerId;
use prices::LatestPrice;
use tokio::sync::{mpsc, watch};

use super::{Delivery, Fanout, PLAYER_EVENT_QUEUE, PlayerEvent, Subscription};

/// The in-process `Fanout`: one `watch` slot for the price that every stream reads, and a bounded queue per stream for
/// its Player's events, held in a registry keyed by Player ID.
#[derive(Clone)]
pub struct Broadcast {
    price: Arc<watch::Sender<Delivery>>,
    players: Arc<Mutex<Registry>>,
}

#[derive(Default)]
struct Registry {
    closed: bool,
    streams: HashMap<PlayerId, Vec<mpsc::Sender<PlayerEvent>>>,
}

impl Broadcast {
    #[must_use]
    pub fn new() -> Self {
        let (price, _) = watch::channel(Delivery::Price(LatestPrice::default()));
        Self {
            price: Arc::new(price),
            players: Arc::default(),
        }
    }

    fn registry(&self) -> std::sync::MutexGuard<'_, Registry> {
        self.players.lock().unwrap_or_else(PoisonError::into_inner)
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
            if matches!(delivery, Delivery::Closed(_)) {
                return false;
            }
            *delivery = Delivery::Price(latest);
            true
        });
    }

    fn notify_player(&self, player: PlayerId, event: PlayerEvent) {
        let mut registry = self.registry();
        let Some(streams) = registry.streams.get_mut(&player) else {
            return;
        };
        // Dropping the sender of a full or abandoned stream closes it once it has read what it holds.
        streams.retain(|stream| stream.try_send(event).is_ok());
        if streams.is_empty() {
            registry.streams.remove(&player);
        }
    }

    fn subscribe(&self, player: Option<PlayerId>) -> Subscription {
        let price = self.price.subscribe();
        let Some(player) = player else {
            return Subscription::new(price, None);
        };
        let (sender, receiver) = mpsc::channel(PLAYER_EVENT_QUEUE);
        let mut registry = self.registry();
        if !registry.closed {
            // A visitor's waiting Player ID may never get an event, so closed streams are swept here as well.
            registry.streams.retain(|_, streams| {
                streams.retain(|stream| !stream.is_closed());
                !streams.is_empty()
            });
            registry.streams.entry(player).or_default().push(sender);
        }
        Subscription::new(price, Some(receiver))
    }

    fn close(&self) {
        let mut registry = self.registry();
        registry.closed = true;
        // The slot closes first, so a stream that sees its queue of Player events end knows the fanout closed.
        self.price
            .send_modify(|delivery| *delivery = Delivery::Closed(delivery.latest()));
        registry.streams.clear();
    }
}

#[cfg(test)]
mod tests {
    use guessing::PlayerId;
    use jiff::Timestamp;

    use super::{Broadcast, Fanout};

    const AT: Timestamp = Timestamp::constant(1_798_761_600, 0);

    fn player(byte: u8) -> PlayerId {
        PlayerId::new(AT, [byte; 10])
    }

    #[test]
    fn a_closed_stream_leaves_nothing_in_the_registry() {
        let fanout = Broadcast::new();
        // Each visitor's stream subscribes under its own waiting Player ID, which never gets an event.
        for byte in 0..100 {
            drop(fanout.subscribe(Some(player(byte))));
        }

        let _open = fanout.subscribe(Some(player(200)));

        let held: usize = fanout.registry().streams.values().map(Vec::len).sum();
        assert_eq!(held, 1, "only the open stream should be held");
    }
}
