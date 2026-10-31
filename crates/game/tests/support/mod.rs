//! What the tests of `game` share.

/// Both store ports' in-memory fakes behind one value, as `sqlite::Database` holds both.
#[derive(Clone, Default)]
pub struct Stores {
    guesses: guessing::InMemory,
    identities: identity::InMemory,
}

impl guessing::Store for Stores {
    async fn place(&self, guess: &guessing::OpenGuess) -> Result<(), guessing::StoreError> {
        self.guesses.place(guess).await
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
        self.identities.player(token).await
    }

    async fn link(
        &self,
        token: identity::TokenHash,
        candidate: guessing::PlayerId,
    ) -> Result<guessing::PlayerId, identity::StoreError> {
        self.identities.link(token, candidate).await
    }
}
