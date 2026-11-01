//! What the tests of `game` share.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
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
    /// Whether every read of a Player, a link and every read of Guesses fails, as an unavailable store's would.
    pub down: Arc<AtomicBool>,
}

impl Stores {
    fn guesses_down(&self) -> Result<(), guessing::StoreError> {
        if self.down.load(Ordering::SeqCst) {
            return Err(guessing::StoreError::Unavailable(Box::new(Down)));
        }
        Ok(())
    }

    fn identities_down(&self) -> Result<(), identity::StoreError> {
        if self.down.load(Ordering::SeqCst) {
            return Err(identity::StoreError::Unavailable(Box::new(Down)));
        }
        Ok(())
    }
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
        self.guesses_down()?;
        self.guesses.open_guesses().await
    }

    async fn standing(
        &self,
        player: guessing::PlayerId,
    ) -> Result<guessing::Standing, guessing::StoreError> {
        self.guesses_down()?;
        self.guesses.standing(player).await
    }
}

impl identity::Store for Stores {
    async fn player(
        &self,
        token: identity::TokenHash,
    ) -> Result<Option<guessing::PlayerId>, identity::StoreError> {
        self.identities_down()?;
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
        self.identities_down()?;
        self.identities.link(token, candidate).await
    }
}
