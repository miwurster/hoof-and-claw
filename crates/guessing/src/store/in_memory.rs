use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use super::{Standing, Store, StoreError};
use crate::{OpenGuess, PlayerId};

/// The hand-written in-memory fake of the store port.
#[derive(Clone, Default)]
pub struct InMemory {
    players: Arc<Mutex<HashMap<PlayerId, Standing>>>,
}

impl InMemory {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<PlayerId, Standing>> {
        self.players.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Store for InMemory {
    fn place(&self, guess: &OpenGuess) -> impl Future<Output = Result<(), StoreError>> + Send {
        let mut players = self.lock();
        let standing = players.entry(guess.player).or_default();
        let placed = if standing.open_guess.is_some() {
            Err(StoreError::OpenGuessExists)
        } else {
            standing.open_guess = Some(*guess);
            Ok(())
        };
        std::future::ready(placed)
    }

    fn standing(
        &self,
        player: PlayerId,
    ) -> impl Future<Output = Result<Standing, StoreError>> + Send {
        std::future::ready(Ok(self.lock().get(&player).copied().unwrap_or_default()))
    }
}
