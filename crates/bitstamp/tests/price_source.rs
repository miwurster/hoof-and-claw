//! The Bitstamp adapter at the `PriceSource` port, against a local fake Bitstamp.

#[expect(
    clippy::unwrap_used,
    reason = "a test fake fails by panicking, as every test does"
)]
mod fake_bitstamp;

use std::time::Duration;

use bitstamp::{Bitstamp, Config};
use fake_bitstamp::FakeBitstamp;
use prices::source::suite::{self, Driver};
use prices::{Price, PriceSource, Report};

/// Short enough that a test about heartbeats, silence or backoff runs in well under a second or two.
fn config(fake: &FakeBitstamp) -> Config {
    Config {
        feed_url: fake.feed_url(),
        ticker_url: fake.ticker_url(),
        heartbeat_interval: Duration::from_millis(100),
        heartbeat_timeout: Duration::from_millis(100),
        silence_limit: Duration::from_millis(1000),
        backoff_min: Duration::from_millis(20),
        backoff_max: Duration::from_millis(160),
    }
}

#[expect(
    clippy::unwrap_used,
    reason = "a test helper fails by panicking, as every test does"
)]
fn start(fake: &FakeBitstamp) -> (Bitstamp, bitstamp::Shutdown) {
    Bitstamp::start(config(fake)).unwrap()
}

struct FakeDriver {
    fake: FakeBitstamp,
    next_trade_id: std::sync::atomic::AtomicU64,
}

impl Driver for FakeDriver {
    async fn sends(&self, price: Price) {
        let id = self
            .next_trade_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let cents = price.cents();
        self.fake
            .trades(id, &format!("{}.{:02}", cents / 100, cents % 100));
    }

    async fn loses_the_connection(&self) {
        self.fake.drops_connections();
        self.fake.wait_for_no_connection().await;
    }

    async fn restores_the_connection(&self) {
        self.fake.wait_for_a_subscribed_connection().await;
    }
}

/// The shared suite's Price source sends no seed: its ticker fails.
async fn suite_setup() -> (Bitstamp, bitstamp::Shutdown, FakeDriver) {
    let fake = FakeBitstamp::start().await;
    let (source, shutdown) = start(&fake);
    let driver = FakeDriver {
        fake,
        next_trade_id: 1.into(),
    };
    (source, shutdown, driver)
}

#[tokio::test]
async fn a_price_the_source_sends_is_received() {
    let (source, _shutdown, driver) = suite_setup().await;
    suite::a_price_the_source_sends_is_received(source, driver).await;
}

#[tokio::test]
async fn losing_the_connection_is_price_delayed_until_it_is_restored() {
    let (source, _shutdown, driver) = suite_setup().await;
    suite::losing_the_connection_is_price_delayed_until_it_is_restored(source, driver).await;
}

/// How long a test waits for a report it expects.
const PATIENCE: Duration = Duration::from_secs(5);

#[expect(
    clippy::expect_used,
    reason = "a test helper fails by panicking, as every test does"
)]
async fn next(source: &mut Bitstamp) -> Option<Report> {
    tokio::time::timeout(PATIENCE, source.next())
        .await
        .expect("the Bitstamp source should report within the patience")
}

/// Asserts that the source reports nothing for `duration`.
#[expect(
    clippy::panic,
    reason = "a test helper fails by panicking, as every test does"
)]
async fn reports_nothing_for(source: &mut Bitstamp, duration: Duration) {
    if let Ok(report) = tokio::time::timeout(duration, source.next()).await {
        panic!("the Bitstamp source should report nothing, but reported {report:?}");
    }
}

fn received(cents: u64) -> Report {
    Report::Received(Price::from_cents(cents))
}

async fn live_without_seed(fake: &FakeBitstamp) -> (Bitstamp, bitstamp::Shutdown) {
    let (mut source, shutdown) = start(fake);
    assert_eq!(next(&mut source).await, Some(Report::Live));
    (source, shutdown)
}

#[tokio::test]
async fn after_subscribing_the_adapter_reads_the_ticker_once_and_delivers_last() {
    let fake = FakeBitstamp::start().await;
    fake.set_ticker(Some("64000.12"));

    let (mut source, _shutdown) = start(&fake);

    assert_eq!(next(&mut source).await, Some(Report::Live));
    assert_eq!(next(&mut source).await, Some(received(6_400_012)));
    fake.wait_for_heartbeats(3).await;
    assert_eq!(fake.ticker_reads(), 1);
}

#[tokio::test]
async fn a_trade_is_received_in_cents_from_its_price_str() {
    let fake = FakeBitstamp::start().await;
    let (mut source, _shutdown) = live_without_seed(&fake).await;

    fake.trades(1, "64000.5");

    assert_eq!(next(&mut source).await, Some(received(6_400_050)));
}

#[tokio::test]
async fn a_trade_with_more_than_two_decimal_places_is_not_received() {
    let fake = FakeBitstamp::start().await;
    let (mut source, _shutdown) = live_without_seed(&fake).await;

    fake.trades(1, "64000.123");
    fake.trades(2, "64000.13");

    assert_eq!(next(&mut source).await, Some(received(6_400_013)));
}

#[tokio::test]
async fn a_ticker_last_with_more_than_two_decimal_places_is_not_received() {
    let fake = FakeBitstamp::start().await;
    fake.set_ticker(Some("64000.123"));
    let (mut source, _shutdown) = live_without_seed(&fake).await;

    reports_nothing_for(&mut source, Duration::from_millis(300)).await;
}

#[tokio::test]
async fn while_every_heartbeat_is_answered_the_price_stays_live() {
    let fake = FakeBitstamp::start().await;
    let (mut source, _shutdown) = live_without_seed(&fake).await;

    reports_nothing_for(&mut source, Duration::from_millis(500)).await;
    assert!(
        fake.heartbeats() >= 3,
        "the adapter should send a heartbeat every interval"
    );
}

#[tokio::test]
async fn an_unanswered_heartbeat_is_price_delayed() {
    let fake = FakeBitstamp::start().await;
    let (mut source, _shutdown) = live_without_seed(&fake).await;

    fake.answers_heartbeats(false);

    assert_eq!(next(&mut source).await, Some(Report::Delayed));
}

#[tokio::test]
async fn an_unconfirmed_subscription_is_never_live() {
    let fake = FakeBitstamp::start().await;
    fake.confirms_subscriptions(false);

    let (mut source, _shutdown) = start(&fake);

    fake.wait_for_opened(2).await;
    reports_nothing_for(&mut source, Duration::from_millis(100)).await;
}

#[tokio::test]
async fn after_the_silence_limit_without_a_trade_the_adapter_reconnects_and_seeds_again() {
    let fake = FakeBitstamp::start().await;
    fake.set_ticker(Some("64000.00"));
    let (mut source, _shutdown) = start(&fake);
    assert_eq!(next(&mut source).await, Some(Report::Live));
    assert_eq!(next(&mut source).await, Some(received(6_400_000)));

    fake.set_ticker(Some("64001.00"));

    assert_eq!(next(&mut source).await, Some(Report::Delayed));
    assert_eq!(next(&mut source).await, Some(Report::Live));
    assert_eq!(next(&mut source).await, Some(received(6_400_100)));
    assert_eq!(fake.opened(), 2);
    assert_eq!(fake.ticker_reads(), 2);
}

#[tokio::test]
async fn trades_keep_a_quiet_subscription_from_counting_as_silent() {
    let fake = FakeBitstamp::start().await;
    let (mut source, _shutdown) = live_without_seed(&fake).await;

    for id in 0..15 {
        fake.trades(id, "64000.00");
        assert_eq!(next(&mut source).await, Some(received(6_400_000)));
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    assert_eq!(fake.opened(), 1);
}

#[tokio::test]
async fn on_a_reconnect_request_the_adapter_subscribes_a_new_connection_before_it_closes_the_old_one()
 {
    use fake_bitstamp::Happened::{Closed, Opened, Subscribed};
    let fake = FakeBitstamp::start().await;
    fake.set_ticker(Some("64000.00"));
    let (mut source, _shutdown) = start(&fake);
    assert_eq!(next(&mut source).await, Some(Report::Live));
    assert_eq!(next(&mut source).await, Some(received(6_400_000)));
    fake.set_ticker(Some("64000.50"));

    fake.requests_reconnect();

    assert_eq!(next(&mut source).await, Some(received(6_400_050)));
    fake.trades(1, "64001.00");
    assert_eq!(next(&mut source).await, Some(received(6_400_100)));
    assert_eq!(
        fake.happened(),
        [
            Opened(1),
            Subscribed(1),
            Opened(2),
            Subscribed(2),
            Closed(1)
        ]
    );
}

#[tokio::test]
async fn a_reconnect_request_whose_new_connection_is_not_subscribed_is_price_delayed() {
    let fake = FakeBitstamp::start().await;
    let (mut source, _shutdown) = live_without_seed(&fake).await;
    fake.confirms_subscriptions(false);

    fake.requests_reconnect();

    assert_eq!(next(&mut source).await, Some(Report::Delayed));
}

#[tokio::test]
async fn after_an_unplanned_disconnect_the_adapter_keeps_retrying_until_it_is_live_again() {
    let fake = FakeBitstamp::start().await;
    let (mut source, _shutdown) = live_without_seed(&fake).await;
    fake.confirms_subscriptions(false);

    fake.drops_connections();
    assert_eq!(next(&mut source).await, Some(Report::Delayed));
    fake.wait_for_opened(6).await;
    fake.confirms_subscriptions(true);

    assert_eq!(next(&mut source).await, Some(Report::Live));
}

#[tokio::test]
async fn closing_the_adapter_closes_the_connection_reports_price_delayed_and_ends_the_source() {
    let fake = FakeBitstamp::start().await;
    let (mut source, shutdown) = live_without_seed(&fake).await;

    shutdown.close().await;

    assert_eq!(next(&mut source).await, Some(Report::Delayed));
    assert_eq!(next(&mut source).await, None);
    fake.wait_for_no_connection().await;
}
