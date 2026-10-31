//! The shared test suite every `Fanout` adapter passes.

use std::time::Duration;

use guessing::{Direction, GuessId, OpenGuess, PlayerId};
use jiff::Timestamp;
use prices::{LatestPrice, Price, ReceivedPrice, Report, Source};

use super::{Event, Fanout, PLAYER_EVENT_QUEUE, PlayerEvent};
use crate::Open;

/// How long a subscriber waits before a test concludes that nothing arrives.
const NOTHING_ARRIVES: Duration = Duration::from_millis(50);

const AT: Timestamp = Timestamp::constant(1_798_761_600, 0);

fn live(cents: u64) -> LatestPrice {
    let mut latest = LatestPrice::default();
    latest.apply(Report::Live);
    latest.apply(Report::Received(Price::from_cents(cents), None));
    latest
}

fn price(cents: u64) -> Event {
    Event::Price(live(cents))
}

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

pub async fn every_subscriber_receives_a_published_price(fanout: impl Fanout) {
    let mut first = fanout.subscribe(None);
    let mut second = fanout.subscribe(Some(player(1)));

    fanout.publish_price(live(6_400_000));

    assert_eq!(first.next().await, Some(price(6_400_000)));
    assert_eq!(second.next().await, Some(price(6_400_000)));
}

pub async fn a_subscriber_that_does_not_read_gets_only_the_newest_price(fanout: impl Fanout) {
    let mut subscription = fanout.subscribe(None);

    fanout.publish_price(live(6_400_000));
    fanout.publish_price(live(6_400_001));

    assert_eq!(subscription.next().await, Some(price(6_400_001)));
    assert!(
        tokio::time::timeout(NOTHING_ARRIVES, subscription.next())
            .await
            .is_err(),
        "the subscriber should get no older price after the newest"
    );
}

pub async fn a_subscriber_gets_nothing_published_before_it_subscribed(fanout: impl Fanout) {
    fanout.publish_price(live(6_400_000));

    fanout.notify_player(player(1), placed(player(1), 10));

    let mut subscription = fanout.subscribe(Some(player(1)));

    assert!(
        tokio::time::timeout(NOTHING_ARRIVES, subscription.next())
            .await
            .is_err(),
        "the subscriber should get nothing published before it subscribed"
    );
}

pub async fn closing_ends_every_subscription(fanout: impl Fanout) {
    let mut visitors = fanout.subscribe(None);
    let mut players = fanout.subscribe(Some(player(1)));

    fanout.close();

    assert_eq!(visitors.next().await, None);
    assert_eq!(players.next().await, None);
    assert_eq!(fanout.subscribe(None).next().await, None);
    assert_eq!(fanout.subscribe(Some(player(1))).next().await, None);
}

pub async fn a_players_event_reaches_every_stream_of_that_player_and_no_other(fanout: impl Fanout) {
    let alice = player(1);
    let mut first_tab = fanout.subscribe(Some(alice));
    let mut second_tab = fanout.subscribe(Some(alice));
    let mut bobs_tab = fanout.subscribe(Some(player(2)));
    let mut visitors_tab = fanout.subscribe(None);

    fanout.notify_player(alice, placed(alice, 10));

    assert_eq!(
        first_tab.next().await,
        Some(Event::Player(placed(alice, 10)))
    );
    assert_eq!(
        second_tab.next().await,
        Some(Event::Player(placed(alice, 10)))
    );
    for (whose, tab) in [("Bob's", &mut bobs_tab), ("a visitor's", &mut visitors_tab)] {
        assert!(
            tokio::time::timeout(NOTHING_ARRIVES, tab.next())
                .await
                .is_err(),
            "{whose} stream should get none of Alice's events"
        );
    }
}

pub async fn a_player_gets_every_event_in_the_order_it_was_published(fanout: impl Fanout) {
    let alice = player(1);
    let mut tab = fanout.subscribe(Some(alice));

    fanout.notify_player(alice, placed(alice, 10));
    fanout.notify_player(alice, placed(alice, 11));

    assert_eq!(tab.next().await, Some(Event::Player(placed(alice, 10))));
    assert_eq!(tab.next().await, Some(Event::Player(placed(alice, 11))));
}

pub async fn a_stream_whose_queue_of_player_events_is_full_is_closed(fanout: impl Fanout) {
    let alice = player(1);
    let mut slow = fanout.subscribe(Some(alice));
    let mut quick = fanout.subscribe(Some(alice));

    for byte in 0..=u8::try_from(PLAYER_EVENT_QUEUE).unwrap() {
        fanout.notify_player(alice, placed(alice, byte));
        assert_eq!(quick.next().await, Some(Event::Player(placed(alice, byte))));
    }

    for byte in 0..u8::try_from(PLAYER_EVENT_QUEUE).unwrap() {
        assert_eq!(slow.next().await, Some(Event::Player(placed(alice, byte))));
    }
    assert_eq!(slow.next().await, None, "the slow stream should be closed");
}
