use std::sync::{Arc, Mutex, PoisonError};

use guessing::{OpenGuess, PlayerId, Resolution, ResolvedGuess, Standing};
use identity::TokenHash;

/// What a controllable store fails, as one state that replaces the one before.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Failing {
    /// Every operation goes through to the real store.
    #[default]
    Nothing,
    /// Every save of resolutions fails as an unavailable store's would.
    Resolutions,
    /// Every operation of every store port fails as an unavailable store's would, reads and writes alike.
    Everything,
}

/// A store that passes every operation to the store it wraps, unless its `StoreControls` make it fail.
#[derive(Clone)]
pub struct ControllableStore<S> {
    store: S,
    failing: Arc<Mutex<Failing>>,
}

/// Drives a `ControllableStore`: sets what it fails.
#[derive(Clone)]
pub struct StoreControls {
    failing: Arc<Mutex<Failing>>,
}

/// Why a controllable store failed: its controls make it fail.
#[derive(Debug, thiserror::Error)]
#[error("the test controls make the store fail")]
pub struct Failed;

/// A controllable store around `store` and the controls that drive it; it starts failing nothing.
#[must_use]
pub fn controllable_store<S>(store: S) -> (ControllableStore<S>, StoreControls) {
    let failing = Arc::new(Mutex::new(Failing::Nothing));
    (
        ControllableStore {
            store,
            failing: Arc::clone(&failing),
        },
        StoreControls { failing },
    )
}

impl StoreControls {
    /// Makes the store fail what `failing` says, until the next call.
    pub fn set(&self, failing: Failing) {
        *self.failing.lock().unwrap_or_else(PoisonError::into_inner) = failing;
    }
}

impl<S> ControllableStore<S> {
    fn failing(&self) -> Failing {
        *self.failing.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn fails_everything(&self) -> bool {
        self.failing() == Failing::Everything
    }
}

fn guessing_unavailable() -> guessing::StoreError {
    guessing::StoreError::Unavailable(Box::new(Failed))
}

fn identity_unavailable() -> identity::StoreError {
    identity::StoreError::Unavailable(Box::new(Failed))
}

impl<S: guessing::Store> guessing::Store for ControllableStore<S> {
    async fn place(&self, guess: &OpenGuess) -> Result<(), guessing::StoreError> {
        if self.fails_everything() {
            return Err(guessing_unavailable());
        }
        self.store.place(guess).await
    }

    async fn resolve(
        &self,
        resolved: &[ResolvedGuess],
    ) -> Result<Vec<Resolution>, guessing::StoreError> {
        if self.failing() != Failing::Nothing {
            return Err(guessing_unavailable());
        }
        self.store.resolve(resolved).await
    }

    async fn open_guesses(&self) -> Result<Vec<OpenGuess>, guessing::StoreError> {
        if self.fails_everything() {
            return Err(guessing_unavailable());
        }
        self.store.open_guesses().await
    }

    async fn standing(&self, player: PlayerId) -> Result<Standing, guessing::StoreError> {
        if self.fails_everything() {
            return Err(guessing_unavailable());
        }
        self.store.standing(player).await
    }
}

impl<S: identity::Store> identity::Store for ControllableStore<S> {
    async fn player(&self, token: TokenHash) -> Result<Option<PlayerId>, identity::StoreError> {
        if self.fails_everything() {
            return Err(identity_unavailable());
        }
        self.store.player(token).await
    }

    async fn link(
        &self,
        token: TokenHash,
        candidate: PlayerId,
    ) -> Result<PlayerId, identity::StoreError> {
        if self.fails_everything() {
            return Err(identity_unavailable());
        }
        self.store.link(token, candidate).await
    }
}
