//! The controllable clock stands still until a command moves it.

use std::time::Duration;

use game::Clock as _;
use jiff::Timestamp;
use test_controls::ControllableClock;

#[test]
fn the_clock_stands_still_until_a_command_moves_it() {
    let clock = ControllableClock::new();

    assert_eq!(clock.now(), clock.now());
}

#[test]
fn advancing_the_clock_moves_it_forward_by_that_much() {
    let clock = ControllableClock::new();
    let before = clock.now();

    clock.advance(Duration::from_millis(60_001)).unwrap();

    assert_eq!(clock.now().duration_since(before).as_millis(), 60_001);
}

#[test]
fn setting_the_clock_puts_it_at_that_time() {
    let clock = ControllableClock::new();
    let time: Timestamp = "2027-03-01T08:30:00.5Z".parse().unwrap();

    clock.set(time);

    assert_eq!(clock.now(), time);
}

#[test]
fn advancing_the_clock_past_the_last_time_it_can_hold_is_refused() {
    let clock = ControllableClock::new();
    clock.set(Timestamp::MAX);

    assert!(clock.advance(Duration::from_secs(1)).is_err());
    assert_eq!(clock.now(), Timestamp::MAX);
}
