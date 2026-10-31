use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use super::{Resolution, Standing, Store, StoreError};
use crate::{OpenGuess, PlayerId, ResolvedGuess, Score};

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

    fn resolve(
        &self,
        resolved: &[ResolvedGuess],
    ) -> impl Future<Output = Result<Vec<Resolution>, StoreError>> + Send {
        let mut players = self.lock();
        let mut saved = Vec::new();
        for resolution in resolved {
            let Some(standing) = players.get_mut(&resolution.guess.player) else {
                continue;
            };
            if standing.open_guess.map(|open| open.id) != Some(resolution.guess.id) {
                continue;
            }
            standing.open_guess = None;
            standing.score = Score::new(standing.score.value() + resolution.outcome.score_change());
            standing.last_resolved = Some(*resolution);
            saved.push(Resolution {
                guess: *resolution,
                score: standing.score,
            });
        }
        std::future::ready(Ok(saved))
    }

    fn standing(
        &self,
        player: PlayerId,
    ) -> impl Future<Output = Result<Standing, StoreError>> + Send {
        std::future::ready(Ok(self.lock().get(&player).copied().unwrap_or_default()))
    }
}
