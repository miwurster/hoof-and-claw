//! The server's configuration, read once from environment variables at start-up.

use std::path::PathBuf;

/// Environment variables without a prefix; the server refuses to start on an invalid value.
#[derive(Debug, clap::Parser)]
#[command(version, about)]
pub(crate) struct Configuration {
    /// The SQLite database file, created if it does not exist.
    #[arg(long, env = "DATABASE_PATH")]
    pub(crate) database_path: PathBuf,

    /// The port to listen on; 0 picks a free one.
    #[arg(long, env = "PORT", default_value_t = 10000)]
    pub(crate) port: u16,

    /// The log filter.
    #[arg(long, env = "RUST_LOG", default_value = "info")]
    pub(crate) rust_log: String,

    /// One log line of text, or one JSON object, per event.
    #[arg(long, env = "LOG_FORMAT", value_enum, default_value_t = LogFormat::Text)]
    pub(crate) log_format: LogFormat,

    /// The commit the server was built from, reported by `/health`.
    #[arg(long, env = "RENDER_GIT_COMMIT", default_value = "dev")]
    pub(crate) build: String,

    /// The directory holding the built app.
    #[arg(long, env = "APP_DIR", default_value = "spa/dist")]
    pub(crate) app_dir: PathBuf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub(crate) enum LogFormat {
    Text,
    Json,
}
