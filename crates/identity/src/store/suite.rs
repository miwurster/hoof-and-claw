//! The shared test suite every adapter of the `identity` store port passes.

use guessing::PlayerId;
use jiff::Timestamp;

use super::Store;
use crate::Token;

const AT: Timestamp = Timestamp::constant(1_798_761_600, 0);

fn player(byte: u8) -> PlayerId {
    PlayerId::new(AT, [byte; 10])
}

pub async fn a_token_without_a_guess_has_no_player(store: impl Store) {
    let token = Token::from_random_bytes([1; 32]);

    assert_eq!(store.player(token.hash()).await.unwrap(), None);
}

pub async fn a_linked_token_belongs_to_its_player(store: impl Store) {
    let token = Token::from_random_bytes([1; 32]);

    let linked = store.link(token.hash(), player(1)).await.unwrap();

    assert_eq!(linked, player(1));
    assert_eq!(store.player(token.hash()).await.unwrap(), Some(player(1)));
}

pub async fn a_token_linked_twice_keeps_the_player_linked_first(store: impl Store) {
    let token = Token::from_random_bytes([1; 32]);
    store.link(token.hash(), player(1)).await.unwrap();

    let linked = store.link(token.hash(), player(2)).await.unwrap();

    assert_eq!(linked, player(1));
    assert_eq!(store.player(token.hash()).await.unwrap(), Some(player(1)));
}

pub async fn tokens_do_not_share_a_player(store: impl Store) {
    let alices = Token::from_random_bytes([1; 32]);
    let bobs = Token::from_random_bytes([2; 32]);

    store.link(alices.hash(), player(1)).await.unwrap();

    assert_eq!(store.player(bobs.hash()).await.unwrap(), None);
}
