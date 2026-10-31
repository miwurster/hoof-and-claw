//! The in-memory fake passes the store port's shared suite.

use futures_executor::block_on;
use identity::InMemory;
use identity::store::suite;

#[test]
fn a_token_without_a_guess_has_no_player() {
    block_on(suite::a_token_without_a_guess_has_no_player(InMemory::new()));
}

#[test]
fn a_linked_token_belongs_to_its_player() {
    block_on(suite::a_linked_token_belongs_to_its_player(InMemory::new()));
}

#[test]
fn a_token_linked_twice_keeps_the_player_linked_first() {
    block_on(suite::a_token_linked_twice_keeps_the_player_linked_first(
        InMemory::new(),
    ));
}

#[test]
fn tokens_do_not_share_a_player() {
    block_on(suite::tokens_do_not_share_a_player(InMemory::new()));
}
