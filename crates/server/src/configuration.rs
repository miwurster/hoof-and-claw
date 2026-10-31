//! The server's configuration, read once from environment variables at start-up.

use std::path::PathBuf;
use std::time::Duration;

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

    /// How often an open stream gets a keepalive comment, in seconds.
    #[arg(long, env = "SSE_KEEPALIVE_SECS", default_value_t = 15)]
    pub(crate) sse_keepalive_secs: u64,

    /// How often a heartbeat goes to Bitstamp, in seconds.
    #[arg(long, env = "PRICE_HEARTBEAT_INTERVAL_SECS", default_value_t = 10)]
    pub(crate) price_heartbeat_interval_secs: u64,

    /// How long Bitstamp has to answer a heartbeat or a subscription, in seconds.
    #[arg(long, env = "PRICE_HEARTBEAT_TIMEOUT_SECS", default_value_t = 5)]
    pub(crate) price_heartbeat_timeout_secs: u64,

    /// How long without a trade before the server reconnects to Bitstamp, in seconds.
    #[arg(long, env = "PRICE_SILENCE_LIMIT_SECS", default_value_t = 90)]
    pub(crate) price_silence_limit_secs: u64,

    /// The first wait before reconnecting after an unplanned disconnect, in seconds.
    #[arg(long, env = "PRICE_BACKOFF_MIN_SECS", default_value_t = 1)]
    pub(crate) price_backoff_min_secs: u64,

    /// The longest wait before reconnecting, in seconds.
    #[arg(long, env = "PRICE_BACKOFF_MAX_SECS", default_value_t = 30)]
    pub(crate) price_backoff_max_secs: u64,

    /// The directory holding the built app.
    #[arg(long, env = "APP_DIR", default_value = "spa/dist")]
    pub(crate) app_dir: PathBuf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub(crate) enum LogFormat {
    Text,
    Json,
}

impl Configuration {
    /// Where Bitstamp is and how the server keeps its connection healthy (ADR 0007).
    #[cfg_attr(
        feature = "test-controls",
        expect(
            dead_code,
            reason = "a build with the test controls runs on the controllable source"
        )
    )]
    pub(crate) fn bitstamp(&self) -> bitstamp::Config {
        bitstamp::Config {
            feed_url: bitstamp::FEED_URL.to_owned(),
            ticker_url: bitstamp::TICKER_URL.to_owned(),
            heartbeat_interval: Duration::from_secs(self.price_heartbeat_interval_secs),
            heartbeat_timeout: Duration::from_secs(self.price_heartbeat_timeout_secs),
            silence_limit: Duration::from_secs(self.price_silence_limit_secs),
            backoff_min: Duration::from_secs(self.price_backoff_min_secs),
            backoff_max: Duration::from_secs(self.price_backoff_max_secs),
        }
    }
}
