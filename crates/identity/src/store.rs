//! The store port of `identity`: links the hash of a token to its Player.

mod in_memory;
#[expect(
    clippy::missing_panics_doc,
    clippy::unwrap_used,
    reason = "a test suite fails by panicking, as every test does"
)]
pub mod suite;

use guessing::PlayerId;

use crate::TokenHash;

pub use in_memory::InMemory;

#[derive(Debug, thiserror::Error)]
#[error("the store is unavailable: {0}")]
pub struct StoreError(#[source] pub Box<dyn std::error::Error + Send + Sync>);

/// Links the hash of a token to the Player it belongs to.
pub trait Store: Clone + Send + Sync + 'static {
    /// The Player linked to the token, or `None` before its first Guess.
    fn player(
        &self,
        token: TokenHash,
    ) -> impl Future<Output = Result<Option<PlayerId>, StoreError>> + Send;

    /// Links the token to `candidate` unless it is linked already, and returns the Player it is linked to.
    ///
    /// Two first Guesses at once from two tabs both get the Player that was linked first (insert or ignore, then read
    /// back).
    fn link(
        &self,
        token: TokenHash,
        candidate: PlayerId,
    ) -> impl Future<Output = Result<PlayerId, StoreError>> + Send;
}
