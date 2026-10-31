//! The shared test suite every adapter of the `guessing` store port passes.

use jiff::{SignedDuration, Timestamp};
use prices::{Price, ReceivedPrice, Source, TradeId};

use super::{Resolution, Standing, Store, StoreError};
use crate::{Direction, GuessId, OpenGuess, PlayerId, ResolvedGuess, Score};

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
            trade_id: Some(TradeId::new(417_000_001)),
            received_at: Timestamp::constant(1_798_761_599, 900_000_000),
        },
        placed_at: PLACED_AT,
    }
}

/// The Guess resolved against a price received 61.5 seconds after it was placed, `cents` away from its Start price.
fn resolved(guess: OpenGuess, cents: i64, trade_id: Option<u64>) -> ResolvedGuess {
    let price = guess.start.price.cents().checked_add_signed(cents).unwrap();
    guess
        .resolve(ReceivedPrice {
            price: Price::from_cents(price),
            source: Source::Bitstamp,
            trade_id: trade_id.map(TradeId::new),
            received_at: guess
                .placed_at
                .checked_add(SignedDuration::from_millis(61_500))
                .unwrap(),
        })
        .unwrap()
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
            last_resolved: None,
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

pub async fn a_resolution_moves_the_score_and_becomes_the_last_resolved_guess(store: impl Store) {
    let alice = player(1);
    let placed = guess(alice, 10, Direction::Up);
    store.place(&placed).await.unwrap();
    let correct = resolved(placed, 100, Some(417_000_002));

    let saved = store.resolve(&[correct]).await.unwrap();

    assert_eq!(
        saved,
        [Resolution {
            guess: correct,
            score: Score::new(1),
        }]
    );
    assert_eq!(
        store.standing(alice).await.unwrap(),
        Standing {
            score: Score::new(1),
            open_guess: None,
            last_resolved: Some(correct),
        }
    );
}

pub async fn a_seed_price_without_a_trade_id_is_kept_as_one(store: impl Store) {
    let alice = player(1);
    let mut placed = guess(alice, 10, Direction::Down);
    placed.start.trade_id = None;
    store.place(&placed).await.unwrap();
    assert_eq!(
        store.standing(alice).await.unwrap().open_guess,
        Some(placed)
    );
    let wrong = resolved(placed, 1, None);

    store.resolve(&[wrong]).await.unwrap();

    assert_eq!(
        store.standing(alice).await.unwrap().last_resolved,
        Some(wrong)
    );
}

pub async fn a_score_can_go_below_0(store: impl Store) {
    let alice = player(1);
    for byte in [10, 11] {
        let placed = guess(alice, byte, Direction::Up);
        store.place(&placed).await.unwrap();
        store.resolve(&[resolved(placed, -1, None)]).await.unwrap();
    }

    assert_eq!(store.standing(alice).await.unwrap().score, Score::new(-2));
}

pub async fn a_resolution_saved_again_changes_nothing(store: impl Store) {
    let alice = player(1);
    let placed = guess(alice, 10, Direction::Up);
    store.place(&placed).await.unwrap();
    let correct = resolved(placed, 100, None);
    store.resolve(&[correct]).await.unwrap();

    let again = store.resolve(&[correct]).await.unwrap();

    assert_eq!(again, []);
    assert_eq!(store.standing(alice).await.unwrap().score, Score::new(1));
}

pub async fn after_a_resolution_the_player_can_place_the_next_guess(store: impl Store) {
    let alice = player(1);
    let first = guess(alice, 10, Direction::Up);
    store.place(&first).await.unwrap();
    let wrong = resolved(first, -100, None);
    store.resolve(&[wrong]).await.unwrap();
    let next = guess(alice, 11, Direction::Down);

    store.place(&next).await.unwrap();

    assert_eq!(
        store.standing(alice).await.unwrap(),
        Standing {
            score: Score::new(-1),
            open_guess: Some(next),
            last_resolved: Some(wrong),
        }
    );
}

pub async fn the_last_resolved_guess_is_the_most_recent_one(store: impl Store) {
    let alice = player(1);
    let first = guess(alice, 10, Direction::Up);
    store.place(&first).await.unwrap();
    store.resolve(&[resolved(first, 100, None)]).await.unwrap();
    let mut second = guess(alice, 11, Direction::Down);
    second.placed_at = PLACED_AT
        .checked_add(SignedDuration::from_secs(120))
        .unwrap();
    store.place(&second).await.unwrap();
    let later = resolved(second, 100, None);

    store.resolve(&[later]).await.unwrap();

    assert_eq!(
        store.standing(alice).await.unwrap().last_resolved,
        Some(later)
    );
}

pub async fn resolutions_saved_together_move_each_players_own_score(store: impl Store) {
    let alice = player(1);
    let bob = player(2);
    let alices = guess(alice, 10, Direction::Up);
    let bobs = guess(bob, 20, Direction::Up);
    store.place(&alices).await.unwrap();
    store.place(&bobs).await.unwrap();
    let correct = resolved(alices, 100, None);
    let wrong = resolved(bobs, -100, None);

    let saved = store.resolve(&[correct, wrong]).await.unwrap();

    assert_eq!(
        saved,
        [
            Resolution {
                guess: correct,
                score: Score::new(1),
            },
            Resolution {
                guess: wrong,
                score: Score::new(-1),
            },
        ]
    );
    assert_eq!(store.standing(alice).await.unwrap().score, Score::new(1));
    assert_eq!(store.standing(bob).await.unwrap().score, Score::new(-1));
}
