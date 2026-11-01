//! The server's configuration, read once from environment variables at start-up.

use std::num::{NonZeroU32, NonZeroUsize};
use std::path::PathBuf;
use std::time::Duration;

use axum::http::HeaderName;

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

    /// How long the server waits on `SIGTERM` for running requests and a running save before it exits, in seconds.
    #[arg(long, env = "SHUTDOWN_DRAIN_SECS", default_value_t = 20)]
    pub(crate) shutdown_drain_secs: u64,

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

    /// Placing a Guess: how many a minute per IP.
    #[arg(long, env = "GUESS_RATE_PER_MINUTE", default_value = "10")]
    pub(crate) guess_rate_per_minute: NonZeroU32,

    /// Placing a Guess: how many at once per IP before the rate applies.
    #[arg(long, env = "GUESS_RATE_BURST", default_value = "10")]
    pub(crate) guess_rate_burst: NonZeroU32,

    /// New identity cookies: how many an hour per IP.
    #[arg(long, env = "COOKIE_RATE_PER_HOUR", default_value = "30")]
    pub(crate) cookie_rate_per_hour: NonZeroU32,

    /// New identity cookies: how many at once per IP before the rate applies.
    #[arg(long, env = "COOKIE_RATE_BURST", default_value = "30")]
    pub(crate) cookie_rate_burst: NonZeroU32,

    /// The header the proxy appends the client's address to; the per-IP limits count by one of its entries.
    #[arg(long, env = "CLIENT_IP_HEADER", default_value = "x-forwarded-for")]
    pub(crate) client_ip_header: HeaderName,

    /// Which entry of `CLIENT_IP_HEADER` is the client's address, counted from the right: 1 is the rightmost.
    #[arg(long, env = "CLIENT_IP_FROM_RIGHT", default_value = "1")]
    pub(crate) client_ip_from_right: NonZeroUsize,

    /// How many streams one IP may hold open at once.
    #[arg(long, env = "MAX_STREAMS_PER_IP", default_value_t = 10)]
    pub(crate) max_streams_per_ip: usize,

    /// How many Player events a stream may hold unread before the server closes it.
    #[arg(long, env = "PLAYER_EVENT_QUEUE", default_value = "16")]
    pub(crate) player_event_queue: NonZeroUsize,

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

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::Configuration;

    fn parse(arguments: &[&str]) -> Configuration {
        Configuration::try_parse_from(
            ["server", "--database-path", "game.db"]
                .iter()
                .chain(arguments),
        )
        .unwrap()
    }

    #[test]
    fn a_stream_holds_16_player_events_by_default() {
        assert_eq!(parse(&[]).player_event_queue.get(), 16);
    }

    #[test]
    fn the_queue_of_player_events_is_configurable() {
        assert_eq!(
            parse(&["--player-event-queue", "4"])
                .player_event_queue
                .get(),
            4
        );
    }
}
