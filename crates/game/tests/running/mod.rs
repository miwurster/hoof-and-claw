//! A running game with a fake Price source, a clock the test moves, the in-memory stores and the recording `Fanout`.

use std::sync::{Arc, Mutex, PoisonError};

use game::{Clock, Game, Recording};
use jiff::{SignedDuration, Timestamp};
use prices::{Price, PriceSource, Report, Source};
use tokio::sync::mpsc;

use crate::support::Stores;

/// 2026-10-31T12:00:00Z.
pub const NOW: Timestamp = Timestamp::constant(1_793_448_000, 0);

#[derive(Clone)]
pub struct MovableClock(Arc<Mutex<Timestamp>>);

impl MovableClock {
    pub fn advance(&self, duration: SignedDuration) {
        let mut now = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        *now = now.saturating_add(duration).unwrap_or(Timestamp::MAX);
    }
}

impl Clock for MovableClock {
    fn now(&self) -> Timestamp {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

struct FakeSource(mpsc::UnboundedReceiver<Report>);

impl PriceSource for FakeSource {
    fn source(&self) -> Source {
        Source::Bitstamp
    }

    async fn next(&mut self) -> Option<Report> {
        self.0.recv().await
    }
}

pub struct Running {
    pub game: Game<MovableClock, Recording, Stores>,
    pub clock: MovableClock,
    pub fanout: Recording,
    /// The game's task that takes in the Price source's reports.
    pub task: tokio::task::JoinHandle<()>,
    reports: mpsc::UnboundedSender<Report>,
}

impl Running {
    pub fn start() -> Self {
        Self::start_with(&Stores::default())
    }

    /// Starts the game on stores the test keeps a handle to.
    pub fn start_with(stores: &Stores) -> Self {
        let clock = MovableClock(Arc::new(Mutex::new(NOW)));
        let fanout = Recording::new();
        let game = Game::new(clock.clone(), fanout.clone(), stores.clone());
        let (reports, receiver) = mpsc::unbounded_channel();
        let task = tokio::spawn({
            let game = game.clone();
            async move { game.run(FakeSource(receiver)).await }
        });
        Self {
            game,
            clock,
            fanout,
            task,
            reports,
        }
    }

    pub async fn report(&self, report: Report) {
        assert!(
            !self.task.is_finished() && self.reports.send(report).is_ok(),
            "the game should still take reports"
        );
        settle().await;
    }

    pub async fn live_at(&self, cents: u64) {
        self.report(Report::Live).await;
        self.receives(cents).await;
    }

    /// The Price source sends a price, which the game receives.
    pub async fn receives(&self, cents: u64) {
        self.report(Report::Received(Price::from_cents(cents), None))
            .await;
    }
}

/// Lets the game's task take in everything sent so far.
pub async fn settle() {
    for _ in 0..10 {
        tokio::task::yield_now().await;
    }
}
