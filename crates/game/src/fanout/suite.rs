//! The shared test suite every `Fanout` adapter passes.

use std::time::Duration;

use prices::{LatestPrice, Price, Report};

use super::Fanout;

/// How long a subscriber waits before a test concludes that nothing arrives.
const NOTHING_ARRIVES: Duration = Duration::from_millis(50);

fn live(cents: u64) -> LatestPrice {
    let mut latest = LatestPrice::default();
    latest.apply(Report::Live);
    latest.apply(Report::Received(Price::from_cents(cents)));
    latest
}

pub async fn every_subscriber_receives_a_published_price(fanout: impl Fanout) {
    let mut first = fanout.subscribe();
    let mut second = fanout.subscribe();

    fanout.publish_price(live(6_400_000));

    assert_eq!(first.next().await, Some(live(6_400_000)));
    assert_eq!(second.next().await, Some(live(6_400_000)));
}

pub async fn a_subscriber_that_does_not_read_gets_only_the_newest_price(fanout: impl Fanout) {
    let mut subscription = fanout.subscribe();

    fanout.publish_price(live(6_400_000));
    fanout.publish_price(live(6_400_001));

    assert_eq!(subscription.next().await, Some(live(6_400_001)));
    assert!(
        tokio::time::timeout(NOTHING_ARRIVES, subscription.next())
            .await
            .is_err(),
        "the subscriber should get no older price after the newest"
    );
}

pub async fn a_subscriber_gets_nothing_published_before_it_subscribed(fanout: impl Fanout) {
    fanout.publish_price(live(6_400_000));

    let mut subscription = fanout.subscribe();

    assert!(
        tokio::time::timeout(NOTHING_ARRIVES, subscription.next())
            .await
            .is_err(),
        "the subscriber should get nothing published before it subscribed"
    );
}

pub async fn closing_ends_every_subscription(fanout: impl Fanout) {
    let mut subscription = fanout.subscribe();

    fanout.close();

    assert_eq!(subscription.next().await, None);
    assert_eq!(fanout.subscribe().next().await, None);
}
