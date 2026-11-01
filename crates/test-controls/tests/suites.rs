//! The controllable clock, Price source and store pass their ports' shared suites.

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

/// Runs each case of a store port's shared suite against a controllable store that fails nothing, around the port's
/// in-memory fake.
macro_rules! shared_suite {
    ($suite:path, $store:expr => $($case:ident),+ $(,)?) => {
        $(
            #[test]
            fn $case() {
                use $suite as suite;
                let (store, _controls) = test_controls::controllable_store($store);
                futures_executor::block_on(suite::$case(store));
            }
        )+
    };
}

mod guessing_store {
    shared_suite!(guessing::store::suite, guessing::InMemory::new() =>
        a_player_who_was_never_stored_stands_at_score_0_with_no_open_guess,
        a_placed_guess_is_the_players_open_guess,
        a_player_with_an_open_guess_cannot_place_another,
        players_do_not_affect_each_other,
        a_resolution_moves_the_score_and_becomes_the_last_resolved_guess,
        a_seed_price_without_a_trade_id_is_kept_as_one,
        a_score_can_go_below_0,
        a_resolution_saved_again_changes_nothing,
        after_a_resolution_the_player_can_place_the_next_guess,
        the_last_resolved_guess_is_the_most_recent_one,
        resolutions_saved_together_move_each_players_own_score,
        the_open_guesses_are_every_players_open_guess_and_no_resolved_one,
    );
}

mod identity_store {
    shared_suite!(identity::store::suite, identity::InMemory::new() =>
        a_token_without_a_guess_has_no_player,
        a_linked_token_belongs_to_its_player,
        a_token_linked_twice_keeps_the_player_linked_first,
        tokens_do_not_share_a_player,
    );
}
