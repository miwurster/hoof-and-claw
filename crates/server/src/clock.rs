//! The system clock, the only place that reads the time of the machine.

use std::time::SystemTime;

use jiff::Timestamp;

/// The real `Clock` of a release build.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(
    all(feature = "test-controls", not(test)),
    expect(
        dead_code,
        reason = "a build with the test controls runs on the controllable clock"
    )
)]
pub(crate) struct SystemClock;

impl game::Clock for SystemClock {
    #[expect(
        clippy::disallowed_methods,
        reason = "the system clock adapter is the one place that reads the machine's time"
    )]
    fn now(&self) -> Timestamp {
        // The machine's time lies within what a `Timestamp` can hold, from year -9999 to 9999.
        Timestamp::try_from(SystemTime::now()).unwrap_or(Timestamp::UNIX_EPOCH)
    }
}

#[cfg(test)]
mod tests {
    use super::SystemClock;

    #[test]
    fn the_system_clocks_time_never_goes_backwards() {
        game::clock::suite::the_time_never_goes_backwards(&SystemClock);
    }
}
