//! The `identity` store port on the table `identity_links`.

use guessing::PlayerId;
use identity::{Store, StoreError, TokenHash};

use crate::Database;

#[derive(Debug, thiserror::Error)]
#[error("the database links a token to {0:?}, which is no Player ID")]
struct Corrupt(String);

fn unavailable(error: impl std::error::Error + Send + Sync + 'static) -> StoreError {
    StoreError::Unavailable(Box::new(error))
}

fn player(id: String) -> Result<PlayerId, StoreError> {
    id.parse().map_err(|_| unavailable(Corrupt(id)))
}

impl Store for Database {
    async fn player(&self, token: TokenHash) -> Result<Option<PlayerId>, StoreError> {
        let hash = token.as_bytes().as_slice();
        sqlx::query_scalar!(
            "SELECT player_id FROM identity_links WHERE token_hash = ?",
            hash
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(unavailable)?
        .map(player)
        .transpose()
    }

    async fn link(&self, token: TokenHash, candidate: PlayerId) -> Result<PlayerId, StoreError> {
        let hash = token.as_bytes().as_slice();
        let candidate = candidate.to_string();
        let mut connection = self.pool.acquire().await.map_err(unavailable)?;
        sqlx::query!(
            "INSERT INTO identity_links (token_hash, player_id) VALUES (?, ?) ON CONFLICT (token_hash) DO NOTHING",
            hash,
            candidate,
        )
        .execute(&mut *connection)
        .await
        .map_err(unavailable)?;
        let linked = sqlx::query_scalar!(
            "SELECT player_id FROM identity_links WHERE token_hash = ?",
            hash
        )
        .fetch_one(&mut *connection)
        .await
        .map_err(unavailable)?;
        player(linked)
    }
}

#[cfg(test)]
mod tests {
    use guessing::PlayerId;
    use identity::{Store, Token};
    use jiff::Timestamp;

    use crate::Database;

    #[tokio::test]
    async fn a_closed_database_is_unavailable() {
        let dir = tempfile::tempdir().unwrap();
        let database = Database::open(&dir.path().join("game.db")).await.unwrap();
        database.close().await;
        let token = Token::from_random_bytes([1; 32]).hash();

        let read = database.player(token).await;
        let linked = database
            .link(token, PlayerId::new(Timestamp::UNIX_EPOCH, [1; 10]))
            .await;

        for error in [read.unwrap_err(), linked.unwrap_err()] {
            assert!(
                error.to_string().starts_with("the store is unavailable: "),
                "{error}"
            );
        }
    }
}
