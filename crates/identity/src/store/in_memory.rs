use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use guessing::PlayerId;

use super::{Store, StoreError};
use crate::TokenHash;

/// The hand-written in-memory fake of the store port.
#[derive(Clone, Default)]
pub struct InMemory {
    links: Arc<Mutex<HashMap<TokenHash, PlayerId>>>,
}

impl InMemory {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<TokenHash, PlayerId>> {
        self.links.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Store for InMemory {
    fn player(
        &self,
        token: TokenHash,
    ) -> impl Future<Output = Result<Option<PlayerId>, StoreError>> + Send {
        std::future::ready(Ok(self.lock().get(&token).copied()))
    }

    fn link(
        &self,
        token: TokenHash,
        candidate: PlayerId,
    ) -> impl Future<Output = Result<PlayerId, StoreError>> + Send {
        std::future::ready(Ok(*self.lock().entry(token).or_insert(candidate)))
    }
}
