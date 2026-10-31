use std::path::Path;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions};

/// The game's database file, migrated, behind exactly one write connection.
pub struct Database {
    pool: SqlitePool,
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
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .map_err(Error::Open)?;
        sqlx::migrate!().run(&pool).await.map_err(Error::Migrate)?;
        Ok(Self { pool })
    }

    /// Waits for the running statement and closes the connection.
    pub async fn close(self) {
        self.pool.close().await;
    }
}
