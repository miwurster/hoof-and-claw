//! The `Clock` port: the domain's time, and only that.

use jiff::Timestamp;

/// Gives the server's time: when a Guess was placed and when it falls due.
pub trait Clock: Clone + Send + Sync + 'static {
    fn now(&self) -> Timestamp;
}

/// The shared test suite every `Clock` adapter passes.
#[expect(
    clippy::missing_panics_doc,
    reason = "a test suite fails by panicking, as every test does"
)]
pub mod suite {
    use super::Clock;

    /// Reading the time twice never gives an earlier time the second time.
    pub fn the_time_never_goes_backwards(clock: &impl Clock) {
        let first = clock.now();
        let second = clock.now();

        assert!(
            second >= first,
            "the clock went back from {first} to {second}"
        );
    }
}
