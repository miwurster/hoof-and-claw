//! The running game with a fake Price source, a clock that stands still and the recording `Fanout`.

use std::time::Duration;

use jiff::Timestamp;
use prices::{Price, Report, ShownPrice, TradeId};
use running::{Running, settle};

#[expect(dead_code, reason = "this binary takes only some of the steps")]
mod running;
mod support;

async fn wait(duration: Duration) {
    tokio::time::sleep(duration).await;
    settle().await;
}

fn latest(cents: Option<u64>, delayed: bool) -> ShownPrice {
    ShownPrice::new(cents.map(Price::from_cents), delayed)
}

#[tokio::test(start_paused = true)]
async fn a_changed_price_after_a_quiet_second_is_published_at_once() {
    let running = Running::start();

    running.live_at(6_400_000).await;

    assert_eq!(running.published_prices(), [6_400_000]);
}

#[tokio::test(start_paused = true)]
async fn a_price_equal_to_the_last_one_published_is_not_published() {
    let running = Running::start();
    running.live_at(6_400_000).await;
    wait(Duration::from_secs(2)).await;

    running.receives(6_400_000).await;
    wait(Duration::from_secs(2)).await;

    assert_eq!(running.published_prices(), [6_400_000]);
}

#[tokio::test(start_paused = true)]
async fn a_second_change_within_one_second_arrives_when_the_second_ends_with_the_newest_price() {
    let running = Running::start();
    running.live_at(6_400_000).await;

    wait(Duration::from_millis(200)).await;
    running.receives(6_400_001).await;
    wait(Duration::from_millis(300)).await;
    running.receives(6_400_002).await;
    wait(Duration::from_millis(499)).await;

    assert_eq!(
        running.published_prices(),
        [6_400_000],
        "no second price within the second"
    );

    wait(Duration::from_millis(1)).await;

    assert_eq!(running.published_prices(), [6_400_000, 6_400_002]);
}

#[tokio::test(start_paused = true)]
async fn two_trades_at_the_same_new_price_within_one_second_publish_it_once() {
    let running = Running::start();
    running.live_at(6_400_000).await;

    wait(Duration::from_millis(200)).await;
    running
        .report(Report::Received(
            Price::from_cents(6_400_001),
            Some(TradeId::new(1)),
        ))
        .await;
    wait(Duration::from_millis(300)).await;
    running
        .report(Report::Received(
            Price::from_cents(6_400_001),
            Some(TradeId::new(2)),
        ))
        .await;
    wait(Duration::from_secs(2)).await;
    running
        .report(Report::Received(
            Price::from_cents(6_400_001),
            Some(TradeId::new(3)),
        ))
        .await;
    wait(Duration::from_secs(2)).await;

    assert_eq!(running.published_prices(), [6_400_000, 6_400_001]);
}

#[tokio::test(start_paused = true)]
async fn the_price_stream_carries_at_most_one_price_a_second() {
    let running = Running::start();
    running.live_at(6_400_000).await;
    wait(Duration::from_millis(50)).await;

    for cents in 6_400_001..6_400_031 {
        wait(Duration::from_millis(100)).await;
        running.receives(cents).await;
    }
    wait(Duration::from_secs(1)).await;

    assert_eq!(
        running.published_prices(),
        [6_400_000, 6_400_009, 6_400_019, 6_400_029, 6_400_030]
    );
}

#[tokio::test(start_paused = true)]
async fn a_change_between_live_and_price_delayed_is_published_at_once() {
    let running = Running::start();
    running.live_at(6_400_000).await;

    running.report(Report::Delayed).await;
    running.report(Report::Live).await;

    assert!(
        running.fanout.published().ends_with(&[
            latest(Some(6_400_000), false),
            latest(Some(6_400_000), true),
            latest(Some(6_400_000), false)
        ]),
        "{:?}",
        running.fanout.published()
    );
}

#[tokio::test(start_paused = true)]
async fn the_snapshot_holds_the_latest_price_and_the_servers_time() {
    let running = Running::start();

    let at_start = running.game.snapshot(None).await.unwrap();
    running.live_at(6_400_000).await;
    let live = running.game.snapshot(None).await.unwrap();

    assert_eq!(at_start.latest, ShownPrice::default());
    assert_eq!(live.latest, latest(Some(6_400_000), false));
    assert_eq!(
        live.server_time,
        "2026-10-31T12:00:00Z".parse::<Timestamp>().unwrap()
    );
}

#[tokio::test(start_paused = true)]
async fn ending_the_streams_closes_every_subscription() {
    let running = Running::start();
    let (_, mut subscription) = running.game.open_stream(&running::alice()).await.unwrap();

    running.game.end_streams();

    assert_eq!(subscription.next().await, None);
}

#[cfg(feature = "test-controls")]
#[tokio::test(start_paused = true)]
async fn forgetting_the_price_returns_to_the_start_state_and_publishes_it() {
    let running = Running::start();
    running.live_at(6_400_000).await;

    running.game.forget_the_price();
    wait(Duration::from_secs(2)).await;
    running.live_at(6_400_000).await;

    assert_eq!(
        running.game.snapshot(None).await.unwrap().latest,
        latest(Some(6_400_000), false)
    );
    assert_eq!(
        running.fanout.published()[2..],
        [
            ShownPrice::default(),
            latest(None, false),
            latest(Some(6_400_000), false)
        ]
    );
}
