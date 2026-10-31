//! The shared test suite every adapter of the `guessing` store port passes.

use jiff::Timestamp;
use prices::{Price, ReceivedPrice, Source};

use super::{Standing, Store, StoreError};
use crate::{Direction, GuessId, OpenGuess, PlayerId, Score};

/// 2027-01-01T00:00:00.123456789Z: a time with nanoseconds, which the store must keep.
const PLACED_AT: Timestamp = Timestamp::constant(1_798_761_600, 123_456_789);

fn player(byte: u8) -> PlayerId {
    PlayerId::new(PLACED_AT, [byte; 10])
}

fn guess(player: PlayerId, byte: u8, direction: Direction) -> OpenGuess {
    OpenGuess {
        id: GuessId::new(PLACED_AT, [byte; 10]),
        player,
        direction,
        start: ReceivedPrice {
            price: Price::from_cents(6_400_012),
            source: Source::Bitstamp,
            received_at: Timestamp::constant(1_798_761_599, 900_000_000),
        },
        placed_at: PLACED_AT,
    }
}

pub async fn a_player_who_was_never_stored_stands_at_score_0_with_no_open_guess(store: impl Store) {
    let standing = store.standing(player(1)).await.unwrap();

    assert_eq!(standing, Standing::default());
    assert_eq!(standing.score, Score::new(0));
}

pub async fn a_placed_guess_is_the_players_open_guess(store: impl Store) {
    let alice = player(1);
    let placed = guess(alice, 10, Direction::Down);

    store.place(&placed).await.unwrap();

    assert_eq!(
        store.standing(alice).await.unwrap(),
        Standing {
            score: Score::new(0),
            open_guess: Some(placed),
        }
    );
}

pub async fn a_player_with_an_open_guess_cannot_place_another(store: impl Store) {
    let alice = player(1);
    let first = guess(alice, 10, Direction::Up);
    store.place(&first).await.unwrap();

    let second = store.place(&guess(alice, 11, Direction::Down)).await;

    assert!(
        matches!(second, Err(StoreError::OpenGuessExists)),
        "the second Guess should be refused, but was {second:?}"
    );
    assert_eq!(store.standing(alice).await.unwrap().open_guess, Some(first));
}

pub async fn players_do_not_affect_each_other(store: impl Store) {
    let alice = player(1);
    let bob = player(2);
    let alices = guess(alice, 10, Direction::Up);
    let bobs = guess(bob, 20, Direction::Down);

    store.place(&alices).await.unwrap();
    store.place(&bobs).await.unwrap();

    assert_eq!(
        store.standing(alice).await.unwrap().open_guess,
        Some(alices)
    );
    assert_eq!(store.standing(bob).await.unwrap().open_guess, Some(bobs));
}
