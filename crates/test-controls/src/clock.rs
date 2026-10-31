use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use jiff::{SignedDuration, Timestamp};

/// The time a fresh controllable clock shows, until a command moves it.
const START: Timestamp = Timestamp::constant(1_798_761_600, 0); // 2027-01-01T00:00:00Z

/// A `Clock` that stands still until a command moves it (ADR 0011).
#[derive(Clone)]
pub struct ControllableClock {
    now: Arc<Mutex<Timestamp>>,
}

#[derive(Debug, thiserror::Error)]
#[error("the clock cannot go past {}", Timestamp::MAX)]
pub struct BeyondTheLastTime;

impl ControllableClock {
    #[must_use]
    pub fn new() -> Self {
        Self {
            now: Arc::new(Mutex::new(START)),
        }
    }

    /// Moves the clock forward by `duration`.
    ///
    /// # Errors
    ///
    /// Refuses a move past the last time the clock can hold, and leaves the clock where it was.
    pub fn advance(&self, duration: Duration) -> Result<(), BeyondTheLastTime> {
        let mut now = self.lock();
        let duration = SignedDuration::try_from(duration).map_err(|_| BeyondTheLastTime)?;
        *now = now.checked_add(duration).map_err(|_| BeyondTheLastTime)?;
        Ok(())
    }

    /// Puts the clock at `time`.
    pub fn set(&self, time: Timestamp) {
        *self.lock() = time;
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Timestamp> {
        self.now.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Default for ControllableClock {
    fn default() -> Self {
        Self::new()
    }
}

impl game::Clock for ControllableClock {
    fn now(&self) -> Timestamp {
        *self.lock()
    }
}
