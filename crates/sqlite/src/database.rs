use std::path::Path;

use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteSynchronous,
};

/// The game's database file, migrated, behind exactly one write connection.
#[derive(Clone)]
pub struct Database {
    pub(crate) pool: SqlitePool,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("cannot open the database: {0}")]
    Open(#[source] sqlx::Error),
    #[error("cannot migrate the database: {0}")]
    Migrate(#[source] sqlx::migrate::MigrateError),
}

impl Database {
    /// Opens the database file, creating it if it does not exist, and runs every migration.
    ///
    /// # Errors
    ///
    /// Fails when the file cannot be opened or created, or a migration fails.
    pub async fn open(path: &Path) -> Result<Self, Error> {
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            // A commit survives a crash of the server; only an OS crash can lose the last ones (ADR 0003).
            .synchronous(SqliteSynchronous::Normal)
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .map_err(Error::Open)?;
        sqlx::migrate!().run(&pool).await.map_err(Error::Migrate)?;
        Ok(Self { pool })
    }

    /// Waits for the running statement and closes the connection, for every clone.
    pub async fn close(&self) {
        self.pool.close().await;
    }
}

#[cfg(test)]
mod tests {
    use crate::Database;

    /// SQLite's code for `synchronous=NORMAL`.
    const NORMAL: i64 = 1;

    #[tokio::test]
    async fn a_commit_waits_for_no_fsync_outside_a_checkpoint() {
        let dir = tempfile::tempdir().unwrap();
        let database = Database::open(&dir.path().join("game.db")).await.unwrap();

        let synchronous: i64 = sqlx::query_scalar("PRAGMA synchronous")
            .fetch_one(&database.pool)
            .await
            .unwrap();

        assert_eq!(
            synchronous, NORMAL,
            "the database should sync only at a checkpoint (ADR 0003)"
        );
    }
}
