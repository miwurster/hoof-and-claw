use std::sync::{Arc, Mutex, PoisonError};

use guessing::{OpenGuess, PlayerId, Resolution, ResolvedGuess, Standing};
use identity::TokenHash;

/// What a controllable store fails, as one state that replaces the one before.
///
/// The variants are ordered so that a later one fails at least as much as an earlier one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Failing {
    /// Every operation goes through to the real store.
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

    /// Fails when the store fails at least what `from` says.
    fn refuses(&self, from: Failing) -> Result<(), Failed> {
        if self.failing() >= from {
            Err(Failed)
        } else {
            Ok(())
        }
    }
}

impl From<Failed> for guessing::StoreError {
    fn from(failed: Failed) -> Self {
        Self::Unavailable(Box::new(failed))
    }
}

impl From<Failed> for identity::StoreError {
    fn from(failed: Failed) -> Self {
        Self::Unavailable(Box::new(failed))
    }
}

impl<S: guessing::Store> guessing::Store for ControllableStore<S> {
    async fn place(&self, guess: &OpenGuess) -> Result<(), guessing::StoreError> {
        self.refuses(Failing::Everything)?;
        self.store.place(guess).await
    }

    async fn resolve(
        &self,
        resolved: &[ResolvedGuess],
    ) -> Result<Vec<Resolution>, guessing::StoreError> {
        self.refuses(Failing::Resolutions)?;
        self.store.resolve(resolved).await
    }

    async fn open_guesses(&self) -> Result<Vec<OpenGuess>, guessing::StoreError> {
        self.refuses(Failing::Everything)?;
        self.store.open_guesses().await
    }

    async fn standing(&self, player: PlayerId) -> Result<Standing, guessing::StoreError> {
        self.refuses(Failing::Everything)?;
        self.store.standing(player).await
    }
}

impl<S: identity::Store> identity::Store for ControllableStore<S> {
    async fn player(&self, token: TokenHash) -> Result<Option<PlayerId>, identity::StoreError> {
        self.refuses(Failing::Everything)?;
        self.store.player(token).await
    }

    async fn link(
        &self,
        token: TokenHash,
        candidate: PlayerId,
    ) -> Result<PlayerId, identity::StoreError> {
        self.refuses(Failing::Everything)?;
        self.store.link(token, candidate).await
    }
}
