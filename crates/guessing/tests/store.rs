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

#[test]
fn a_resolution_moves_the_score_and_becomes_the_last_resolved_guess() {
    block_on(
        suite::a_resolution_moves_the_score_and_becomes_the_last_resolved_guess(InMemory::new()),
    );
}

#[test]
fn a_seed_price_without_a_trade_id_is_kept_as_one() {
    block_on(suite::a_seed_price_without_a_trade_id_is_kept_as_one(
        InMemory::new(),
    ));
}

#[test]
fn a_score_can_go_below_0() {
    block_on(suite::a_score_can_go_below_0(InMemory::new()));
}

#[test]
fn a_resolution_saved_again_changes_nothing() {
    block_on(suite::a_resolution_saved_again_changes_nothing(
        InMemory::new(),
    ));
}

#[test]
fn after_a_resolution_the_player_can_place_the_next_guess() {
    block_on(suite::after_a_resolution_the_player_can_place_the_next_guess(InMemory::new()));
}

#[test]
fn the_last_resolved_guess_is_the_most_recent_one() {
    block_on(suite::the_last_resolved_guess_is_the_most_recent_one(
        InMemory::new(),
    ));
}

#[test]
fn resolutions_saved_together_move_each_players_own_score() {
    block_on(suite::resolutions_saved_together_move_each_players_own_score(InMemory::new()));
}

#[test]
fn the_open_guesses_are_every_players_open_guess_and_no_resolved_one() {
    block_on(
        suite::the_open_guesses_are_every_players_open_guess_and_no_resolved_one(InMemory::new()),
    );
}
