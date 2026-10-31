//! The in-memory fake passes the store port's shared suite.

use futures_executor::block_on;
use guessing::InMemory;
use guessing::store::suite;

#[test]
fn a_player_who_was_never_stored_stands_at_score_0_with_no_open_guess() {
    block_on(
        suite::a_player_who_was_never_stored_stands_at_score_0_with_no_open_guess(InMemory::new()),
    );
}

#[test]
fn a_placed_guess_is_the_players_open_guess() {
    block_on(suite::a_placed_guess_is_the_players_open_guess(
        InMemory::new(),
    ));
}

#[test]
fn a_player_with_an_open_guess_cannot_place_another() {
    block_on(suite::a_player_with_an_open_guess_cannot_place_another(
        InMemory::new(),
    ));
}

#[test]
fn players_do_not_affect_each_other() {
    block_on(suite::players_do_not_affect_each_other(InMemory::new()));
}
