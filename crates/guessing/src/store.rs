//! The store port of `guessing`: keeps Players, their Guesses and Scores.

mod in_memory;
#[expect(
    clippy::missing_panics_doc,
    clippy::unwrap_used,
    reason = "a test suite fails by panicking, as every test does"
)]
pub mod suite;

use crate::{OpenGuess, PlayerId, ResolvedGuess, Score};

pub use in_memory::InMemory;

/// Where a Player stands: their Score, their Open guess and their most recent Resolved guess, if any.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Standing {
    pub score: Score,
    pub open_guess: Option<OpenGuess>,
    pub last_resolved: Option<ResolvedGuess>,
}

/// A saved resolution: the Resolved guess and its Player's Score after it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Resolution {
    pub guess: ResolvedGuess,
    pub score: Score,
}

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("the Player already has an Open guess")]
    OpenGuessExists,
    #[error("the store is unavailable: {0}")]
    Unavailable(#[source] Box<dyn std::error::Error + Send + Sync>),
}

/// Keeps Players, their Guesses and Scores.
pub trait Store: Clone + Send + Sync + 'static {
    /// Stores the Guess as the Player's Open guess, and the Player with Score 0 if this is their first Guess, in one
    /// transaction.
    ///
    /// Refuses with `OpenGuessExists` and stores nothing when the Player already has an Open guess.
    fn place(&self, guess: &OpenGuess) -> impl Future<Output = Result<(), StoreError>> + Send;

    /// Resolves the Guesses and moves each Player's Score by its Outcome, all in one transaction, and answers the
    /// resolutions it saved, in the order given.
    ///
    /// Only a Guess that is still open is resolved, so saving a resolution again changes nothing and answers none
    /// for it.
    fn resolve(
        &self,
        resolved: &[ResolvedGuess],
    ) -> impl Future<Output = Result<Vec<Resolution>, StoreError>> + Send;

    /// Where the Player stands; a Player who was never stored stands at Score 0 with no Open guess.
    fn standing(
        &self,
        player: PlayerId,
    ) -> impl Future<Output = Result<Standing, StoreError>> + Send;
}
