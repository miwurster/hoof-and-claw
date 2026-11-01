//! What the tests of `game` share.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use tokio::sync::oneshot;

/// Tells the test that a read is held, and waits for the test to let it go on.
pub type Hold = (oneshot::Sender<()>, oneshot::Receiver<()>);

/// Both store ports' in-memory fakes behind one value, as `sqlite::Database` holds both.
#[derive(Clone, Default)]
pub struct Stores {
    guesses: guessing::InMemory,
    identities: identity::InMemory,
    /// How many of the next saves of resolutions fail, as an unavailable store's would.
    pub failing_resolutions: Arc<AtomicUsize>,
    /// Holds the next read of a token's Player after it has read: tells the first sender once it is held, and goes on
    /// when the second receiver gets a value.
    pub held_player_read: Arc<Mutex<Option<Hold>>>,
}

#[derive(Debug, thiserror::Error)]
#[error("the store is down")]
struct Down;

impl guessing::Store for Stores {
    async fn place(&self, guess: &guessing::OpenGuess) -> Result<(), guessing::StoreError> {
        self.guesses.place(guess).await
    }

    async fn resolve(
        &self,
        resolved: &[guessing::ResolvedGuess],
    ) -> Result<Vec<guessing::Resolution>, guessing::StoreError> {
        let failing =
            self.failing_resolutions
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |left| {
                    left.checked_sub(1)
                });
        if failing.is_ok() {
            return Err(guessing::StoreError::Unavailable(Box::new(Down)));
        }
        self.guesses.resolve(resolved).await
    }

    async fn open_guesses(&self) -> Result<Vec<guessing::OpenGuess>, guessing::StoreError> {
        self.guesses.open_guesses().await
    }

    async fn standing(
        &self,
        player: guessing::PlayerId,
    ) -> Result<guessing::Standing, guessing::StoreError> {
        self.guesses.standing(player).await
    }
}

impl identity::Store for Stores {
    async fn player(
        &self,
        token: identity::TokenHash,
    ) -> Result<Option<guessing::PlayerId>, identity::StoreError> {
        let player = self.identities.player(token).await;
        let held = self
            .held_player_read
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        if let Some((held, go_on)) = held {
            let _ = held.send(());
            let _ = go_on.await;
        }
        player
    }

    async fn link(
        &self,
        token: identity::TokenHash,
        candidate: guessing::PlayerId,
    ) -> Result<guessing::PlayerId, identity::StoreError> {
        self.identities.link(token, candidate).await
    }
}
