//! The running game with a fake Price source, a fixed clock and the recording `Fanout`.

use std::time::Duration;

use game::{Clock, Game, Recording};
use jiff::Timestamp;
use prices::{LatestPrice, Price, PriceSource, Report};
use tokio::sync::mpsc;

/// 2026-10-31T12:00:00Z.
const NOW: Timestamp = Timestamp::constant(1_793_448_000, 0);

#[derive(Clone)]
struct FixedClock(Timestamp);

impl Clock for FixedClock {
    fn now(&self) -> Timestamp {
        self.0
    }
}

struct FakeSource(mpsc::UnboundedReceiver<Report>);

impl PriceSource for FakeSource {
    async fn next(&mut self) -> Option<Report> {
        self.0.recv().await
    }
}

struct Running {
    game: Game<FixedClock, Recording>,
    fanout: Recording,
    reports: mpsc::UnboundedSender<Report>,
}

impl Running {
    fn start() -> Self {
        let fanout = Recording::new();
        let game = Game::new(FixedClock(NOW), fanout.clone());
        let (reports, receiver) = mpsc::unbounded_channel();
        tokio::spawn({
            let game = game.clone();
            async move { game.run(FakeSource(receiver)).await }
        });
        Self {
            game,
            fanout,
            reports,
        }
    }

    async fn report(&self, report: Report) {
        assert!(
            self.reports.send(report).is_ok(),
            "the game should still take reports"
        );
        settle().await;
    }

    async fn live_at(&self, cents: u64) {
        self.report(Report::Live).await;
        self.report(Report::Received(price(cents))).await;
    }

    fn published_prices(&self) -> Vec<u64> {
        self.fanout
            .published()
            .iter()
            .filter(|latest| !latest.is_delayed())
            .filter_map(|latest| latest.price().map(Price::cents))
            .collect()
    }
}

/// Lets the game's task take in everything sent so far.
async fn settle() {
    for _ in 0..10 {
        tokio::task::yield_now().await;
    }
}

async fn wait(duration: Duration) {
    tokio::time::sleep(duration).await;
    settle().await;
}

fn price(cents: u64) -> Price {
    Price::from_cents(cents)
}

fn latest(cents: Option<u64>, delayed: bool) -> LatestPrice {
    let mut latest = LatestPrice::default();
    latest.apply(if delayed {
        Report::Delayed
    } else {
        Report::Live
    });
    if let Some(cents) = cents {
        latest.apply(Report::Received(price(cents)));
    }
    latest
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

    running.report(Report::Received(price(6_400_000))).await;
    wait(Duration::from_secs(2)).await;

    assert_eq!(running.published_prices(), [6_400_000]);
}

#[tokio::test(start_paused = true)]
async fn a_second_change_within_one_second_arrives_when_the_second_ends_with_the_newest_price() {
    let running = Running::start();
    running.live_at(6_400_000).await;

    wait(Duration::from_millis(200)).await;
    running.report(Report::Received(price(6_400_001))).await;
    wait(Duration::from_millis(300)).await;
    running.report(Report::Received(price(6_400_002))).await;
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
async fn the_price_stream_carries_at_most_one_price_a_second() {
    let running = Running::start();
    running.live_at(6_400_000).await;
    wait(Duration::from_millis(50)).await;

    for cents in 6_400_001..6_400_031 {
        wait(Duration::from_millis(100)).await;
        running.report(Report::Received(price(cents))).await;
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

    let at_start = running.game.snapshot();
    running.live_at(6_400_000).await;
    let live = running.game.snapshot();

    assert_eq!(at_start.latest, LatestPrice::default());
    assert_eq!(live.latest, latest(Some(6_400_000), false));
    assert_eq!(
        live.server_time,
        "2026-10-31T12:00:00Z".parse::<Timestamp>().unwrap()
    );
}

#[tokio::test(start_paused = true)]
async fn ending_the_streams_closes_every_subscription() {
    let running = Running::start();
    let mut subscription = running.game.subscribe();

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
        running.game.snapshot().latest,
        latest(Some(6_400_000), false)
    );
    assert_eq!(
        running.fanout.published()[2..],
        [
            LatestPrice::default(),
            latest(None, false),
            latest(Some(6_400_000), false)
        ]
    );
}
