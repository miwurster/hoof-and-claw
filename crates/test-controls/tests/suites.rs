//! The controllable clock and Price source pass their ports' shared suites.

use futures_executor::block_on;
use prices::Price;
use prices::source::suite::{self, Driver};
use test_controls::{ControllableClock, SourceControls, controllable_source};

struct Controls(SourceControls);

impl Driver for Controls {
    async fn sends(&self, price: Price) {
        self.0.send(price);
    }

    async fn loses_the_connection(&self) {
        self.0.set_connected(false);
    }

    async fn restores_the_connection(&self) {
        self.0.set_connected(true);
    }
}

#[test]
fn a_price_the_source_sends_is_received() {
    let (source, controls) = controllable_source();
    block_on(suite::a_price_the_source_sends_is_received(
        source,
        Controls(controls),
    ));
}

#[test]
fn losing_the_connection_is_price_delayed_until_it_is_restored() {
    let (source, controls) = controllable_source();
    block_on(
        suite::losing_the_connection_is_price_delayed_until_it_is_restored(
            source,
            Controls(controls),
        ),
    );
}

#[test]
fn the_controllable_clocks_time_never_goes_backwards() {
    game::clock::suite::the_time_never_goes_backwards(&ControllableClock::new());
}
