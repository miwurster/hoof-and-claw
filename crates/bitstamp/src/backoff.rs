//! The wait before the next attempt to connect after an unplanned disconnect (ADR 0007).

use std::time::Duration;

/// Starts at the shortest wait and doubles it up to the longest; jitter takes up to a quarter off each wait.
#[derive(Debug)]
pub(crate) struct Backoff {
    shortest: Duration,
    longest: Duration,
    step: Duration,
}

impl Backoff {
    pub(crate) const fn new(shortest: Duration, longest: Duration) -> Self {
        Self {
            shortest,
            longest,
            step: shortest,
        }
    }

    /// The wait before the next attempt; `random` picks the jitter, from a quarter off at 0 to none at `u32::MAX`.
    pub(crate) fn next_wait(&mut self, random: u32) -> Duration {
        let step = self.step;
        self.step = step.saturating_mul(2).min(self.longest);
        let quarter = step / 4;
        let off = quarter.mul_f64(f64::from(u32::MAX - random) / f64::from(u32::MAX));
        step.saturating_sub(off)
    }

    /// A connection succeeded, so the next unplanned disconnect waits the shortest time again.
    pub(crate) const fn reset(&mut self) {
        self.step = self.shortest;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECOND: Duration = Duration::from_secs(1);

    fn waits(backoff: &mut Backoff, random: u32, attempts: usize) -> Vec<Duration> {
        (0..attempts).map(|_| backoff.next_wait(random)).collect()
    }

    #[test]
    fn the_wait_starts_at_one_second_and_doubles_up_to_thirty_seconds() {
        let mut backoff = Backoff::new(SECOND, 30 * SECOND);

        let secs: Vec<u64> = waits(&mut backoff, u32::MAX, 8)
            .iter()
            .map(Duration::as_secs)
            .collect();

        assert_eq!(secs, [1, 2, 4, 8, 16, 30, 30, 30]);
    }

    #[test]
    fn jitter_shortens_a_wait_by_at_most_a_quarter() {
        let mut backoff = Backoff::new(SECOND, 30 * SECOND);

        assert_eq!(backoff.next_wait(0), Duration::from_millis(750));
        assert_eq!(backoff.next_wait(0), Duration::from_millis(1500));
        let halfway = backoff.next_wait(u32::MAX / 2);
        assert!(
            halfway > Duration::from_millis(3000) && halfway < 4 * SECOND,
            "{halfway:?}"
        );
    }

    #[test]
    fn after_a_connection_the_wait_starts_again_at_one_second() {
        let mut backoff = Backoff::new(SECOND, 30 * SECOND);
        waits(&mut backoff, u32::MAX, 4);

        backoff.reset();

        assert_eq!(backoff.next_wait(u32::MAX), SECOND);
    }
}
