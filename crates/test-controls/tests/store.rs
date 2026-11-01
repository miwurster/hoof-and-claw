//! The controllable store fails what its controls say, as an unavailable store would, and nothing else.

use futures_executor::block_on;
use guessing::{Direction, GuessId, OpenGuess, PlayerId, ResolvedGuess, Store as _, StoreError};
use identity::{Store as _, Token};
use jiff::{SignedDuration, Timestamp};
use prices::{Price, ReceivedPrice, Source};
use test_controls::{ControllableStore, Failing, controllable_store};

const PLACED_AT: Timestamp = Timestamp::constant(1_798_761_600, 0);

#[derive(Clone)]
struct Joined {
    guessing: guessing::InMemory,
    identity: identity::InMemory,
}

impl guessing::Store for Joined {
    async fn place(&self, guess: &OpenGuess) -> Result<(), StoreError> {
        self.guessing.place(guess).await
    }

    async fn resolve(
        &self,
        resolved: &[ResolvedGuess],
    ) -> Result<Vec<guessing::Resolution>, StoreError> {
        self.guessing.resolve(resolved).await
    }

    async fn open_guesses(&self) -> Result<Vec<OpenGuess>, StoreError> {
        self.guessing.open_guesses().await
    }

    async fn standing(&self, player: PlayerId) -> Result<guessing::Standing, StoreError> {
        self.guessing.standing(player).await
    }
}

impl identity::Store for Joined {
    async fn player(
        &self,
        token: identity::TokenHash,
    ) -> Result<Option<PlayerId>, identity::StoreError> {
        self.identity.player(token).await
    }

    async fn link(
        &self,
        token: identity::TokenHash,
        candidate: PlayerId,
    ) -> Result<PlayerId, identity::StoreError> {
        self.identity.link(token, candidate).await
    }
}

fn store() -> (ControllableStore<Joined>, test_controls::StoreControls) {
    controllable_store(Joined {
        guessing: guessing::InMemory::new(),
        identity: identity::InMemory::new(),
    })
}

fn price(cents: u64, received_at: Timestamp) -> ReceivedPrice {
    ReceivedPrice {
        price: Price::from_cents(cents),
        source: Source::Bitstamp,
        trade_id: None,
        received_at,
    }
}

fn guess(byte: u8) -> OpenGuess {
    OpenGuess {
        id: GuessId::new(PLACED_AT, [byte; 10]),
        player: PlayerId::new(PLACED_AT, [byte; 10]),
        direction: Direction::Up,
        start: price(6_400_000, PLACED_AT),
        placed_at: PLACED_AT,
    }
}

fn resolved(guess: OpenGuess) -> ResolvedGuess {
    let at = PLACED_AT
        .checked_add(SignedDuration::from_secs(61))
        .unwrap();
    guess.resolve(price(6_400_100, at)).unwrap()
}

#[test]
fn a_store_that_fails_resolutions_refuses_to_save_them_and_serves_everything_else() {
    block_on(async {
        let (store, controls) = store();
        let token = Token::from_random_bytes([1; 32]);
        controls.set(Failing::Resolutions);

        store.place(&guess(1)).await.unwrap();
        store.link(token.hash(), guess(1).player).await.unwrap();
        let refused = store.resolve(&[resolved(guess(1))]).await;

        assert!(matches!(refused, Err(StoreError::Unavailable(_))));
        assert_eq!(store.open_guesses().await.unwrap(), vec![guess(1)]);
        assert_eq!(
            store.player(token.hash()).await.unwrap(),
            Some(guess(1).player)
        );
    });
}

#[test]
fn a_store_that_fails_everything_refuses_every_operation_of_both_ports() {
    block_on(async {
        let (store, controls) = store();
        let token = Token::from_random_bytes([1; 32]);
        store.place(&guess(1)).await.unwrap();
        controls.set(Failing::Everything);

        assert!(matches!(
            store.place(&guess(2)).await,
            Err(StoreError::Unavailable(_))
        ));
        assert!(matches!(
            store.resolve(&[resolved(guess(1))]).await,
            Err(StoreError::Unavailable(_))
        ));
        assert!(matches!(
            store.open_guesses().await,
            Err(StoreError::Unavailable(_))
        ));
        assert!(matches!(
            store.standing(guess(1).player).await,
            Err(StoreError::Unavailable(_))
        ));
        assert!(matches!(
            store.player(token.hash()).await,
            Err(identity::StoreError::Unavailable(_))
        ));
        assert!(matches!(
            store.link(token.hash(), guess(1).player).await,
            Err(identity::StoreError::Unavailable(_))
        ));
    });
}

#[test]
fn a_store_that_recovers_saves_what_it_refused_before() {
    block_on(async {
        let (store, controls) = store();
        store.place(&guess(1)).await.unwrap();
        controls.set(Failing::Everything);
        assert!(store.resolve(&[resolved(guess(1))]).await.is_err());

        controls.set(Failing::Nothing);
        let saved = store.resolve(&[resolved(guess(1))]).await.unwrap();

        assert_eq!(saved.len(), 1);
        assert_eq!(store.open_guesses().await.unwrap(), vec![]);
    });
}
