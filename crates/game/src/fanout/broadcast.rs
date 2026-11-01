use std::collections::HashMap;
use std::num::NonZeroUsize;
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
    /// How many Player events a stream may hold unread before it is closed.
    queue: usize,
}

#[derive(Default)]
struct Registry {
    closed: bool,
    streams: HashMap<PlayerId, Vec<mpsc::Sender<PlayerEvent>>>,
}

impl Broadcast {
    /// A fanout whose streams each hold up to `queue` Player events unread.
    #[must_use]
    pub fn new(queue: NonZeroUsize) -> Self {
        Self::holding(queue.get())
    }

    fn holding(queue: usize) -> Self {
        let (price, _) = watch::channel(Delivery::Price(LatestPrice::default()));
        Self {
            price: Arc::new(price),
            players: Arc::default(),
            queue,
        }
    }

    fn registry(&self) -> std::sync::MutexGuard<'_, Registry> {
        self.players.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Streams hold `PLAYER_EVENT_QUEUE` Player events.
impl Default for Broadcast {
    fn default() -> Self {
        Self::holding(PLAYER_EVENT_QUEUE)
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

    fn subscribe(&self, player: PlayerId) -> Subscription {
        let price = self.price.subscribe();
        let (sender, receiver) = mpsc::channel(self.queue);
        let mut registry = self.registry();
        if !registry.closed {
            // A visitor's waiting Player ID may never get an event, so closed streams are swept here as well.
            registry.streams.retain(|_, streams| {
                streams.retain(|stream| !stream.is_closed());
                !streams.is_empty()
            });
            registry.streams.entry(player).or_default().push(sender);
        }
        Subscription::new(price, receiver)
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
    use std::num::NonZeroUsize;
    use std::time::Duration;

    use guessing::{Direction, GuessId, OpenGuess, PlayerId};
    use jiff::Timestamp;
    use prices::{Price, ReceivedPrice, Source};

    use super::{Broadcast, Fanout, PlayerEvent};
    use crate::{Event, Open};

    const AT: Timestamp = Timestamp::constant(1_798_761_600, 0);

    fn player(byte: u8) -> PlayerId {
        PlayerId::new(AT, [byte; 10])
    }

    fn placed(player: PlayerId, byte: u8) -> PlayerEvent {
        PlayerEvent::GuessPlaced(Open {
            guess: OpenGuess {
                id: GuessId::new(AT, [byte; 10]),
                player,
                direction: Direction::Up,
                start: ReceivedPrice {
                    price: Price::from_cents(6_400_000),
                    source: Source::Bitstamp,
                    trade_id: None,
                    received_at: AT,
                },
                placed_at: AT,
            },
            remaining: Duration::from_secs(60),
        })
    }

    #[test]
    fn a_closed_stream_leaves_nothing_in_the_registry() {
        let fanout = Broadcast::default();
        // Each visitor's stream subscribes under its own waiting Player ID, which never gets an event.
        for byte in 0..100 {
            drop(fanout.subscribe(player(byte)));
        }

        let _open = fanout.subscribe(player(200));

        let held: usize = fanout.registry().streams.values().map(Vec::len).sum();
        assert_eq!(held, 1, "only the open stream should be held");
    }

    #[tokio::test]
    async fn a_stream_holds_as_many_player_events_as_the_queue_it_was_given() {
        let fanout = Broadcast::new(NonZeroUsize::new(2).unwrap());
        let alice = player(1);
        let mut slow = fanout.subscribe(alice);

        for byte in 0..3 {
            fanout.notify_player(alice, placed(alice, byte));
        }

        assert_eq!(slow.next().await, Some(Event::Player(placed(alice, 0))));
        assert_eq!(slow.next().await, Some(Event::Player(placed(alice, 1))));
        assert_eq!(slow.next().await, None, "the slow stream should be closed");
    }
}
