//! Every change between live and Price delayed is a log event with the reason, as the Operator reads it.
//!
//! A test binary of its own, because `tracing` caches whether a log event is enabled across the threads of one
//! process, so another test's thread without a subscriber can hide this test's events.

#[expect(
    clippy::unwrap_used,
    dead_code,
    reason = "a test fake fails by panicking; this binary uses only part of it"
)]
mod fake_bitstamp;

use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bitstamp::{Bitstamp, Config};
use fake_bitstamp::FakeBitstamp;
use prices::{PriceSource as _, Report};

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
    clippy::expect_used,
    reason = "a test helper fails by panicking, as every test does"
)]
async fn next(source: &mut Bitstamp) -> Option<Report> {
    tokio::time::timeout(Duration::from_secs(5), source.next())
        .await
        .expect("the Bitstamp source should report within the patience")
}

/// The log lines written while the guard lives, on this thread; `tokio::test` runs every task on it.
#[derive(Clone, Default)]
struct CapturedLog(Arc<Mutex<Vec<u8>>>);

impl Write for CapturedLog {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.lines().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl CapturedLog {
    fn lines(&self) -> std::sync::MutexGuard<'_, Vec<u8>> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn start() -> (Self, tracing::subscriber::DefaultGuard) {
        let log = Self::default();
        let writer = log.clone();
        let subscriber = tracing_subscriber::fmt()
            .with_ansi(false)
            .with_writer(move || writer.clone())
            .finish();
        (log, tracing::subscriber::set_default(subscriber))
    }

    /// The changes between live and Price delayed, each with its reason.
    fn changes(&self) -> Vec<String> {
        let text = String::from_utf8_lossy(&self.lines()).into_owned();
        text.lines()
            .filter_map(|line| {
                if line.ends_with("price live") {
                    Some("live".to_owned())
                } else if line.contains("price delayed") {
                    let reason = line.split("reason=").nth(1).unwrap_or("no reason");
                    Some(format!("delayed: {reason}"))
                } else {
                    None
                }
            })
            .collect()
    }
}

async fn until(source: &mut Bitstamp, report: Report) {
    while next(source).await.is_some_and(|next| next != report) {}
}

#[tokio::test]
async fn every_change_between_live_and_price_delayed_is_logged_with_its_reason() {
    let (log, _guard) = CapturedLog::start();
    let fake = FakeBitstamp::start().await;
    let (mut source, shutdown) = Bitstamp::start(config(&fake)).unwrap();
    until(&mut source, Report::Live).await;

    fake.drops_connections();
    until(&mut source, Report::Delayed).await;
    until(&mut source, Report::Live).await;

    fake.confirms_subscriptions(false);
    fake.requests_reconnect();
    until(&mut source, Report::Delayed).await;
    fake.confirms_subscriptions(true);
    until(&mut source, Report::Live).await;

    fake.answers_heartbeats(false);
    until(&mut source, Report::Delayed).await;
    fake.answers_heartbeats(true);
    until(&mut source, Report::Live).await;

    until(&mut source, Report::Delayed).await;
    until(&mut source, Report::Live).await;

    shutdown.close().await;

    assert_eq!(
        log.changes(),
        [
            "live",
            "delayed: disconnected",
            "live",
            "delayed: not subscribed",
            "live",
            "delayed: heartbeat unanswered",
            "live",
            "delayed: silence",
            "live",
            "delayed: shutting down",
        ]
    );
}
