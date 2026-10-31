//! The events of the stream and the snapshot it opens with (ADR 0014).

use serde::Serialize;
use ts_rs::TS;

/// Every event on the stream, told apart by `type`.
#[derive(Clone, Debug, Serialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export)]
pub enum ServerEvent {
    Snapshot(Snapshot),
    Price(PriceChanged),
}

/// Everything a tab shows, sent first on every stream and answered by `GET /api/snapshot`.
#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Snapshot {
    /// The commit the server was built from.
    pub build: String,
    /// The server's time, an RFC 3339 timestamp.
    pub server_time: String,
    /// The latest price in cents, or `null` while the server has received none since it started.
    #[ts(type = "number | null")]
    pub price: Option<u64>,
    pub delayed: bool,
    #[ts(type = "number")]
    pub score: i64,
    pub open_guess: Option<OpenGuess>,
    pub last_resolved: Option<ResolvedGuess>,
}

/// The latest price changed, or the price became live or Price delayed.
#[derive(Clone, Copy, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PriceChanged {
    #[ts(type = "number | null")]
    pub price: Option<u64>,
    pub delayed: bool,
}

/// A Player's Open guess.
#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct OpenGuess {
    pub id: String,
    pub direction: Direction,
    #[ts(type = "number")]
    pub start_price: u64,
    /// The time left until the Guess falls due, 0 once it has.
    #[ts(type = "number")]
    pub remaining_ms: u64,
}

/// A Player's most recent Resolved guess.
#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ResolvedGuess {
    pub id: String,
    pub direction: Direction,
    #[ts(type = "number")]
    pub start_price: u64,
    #[ts(type = "number")]
    pub end_price: u64,
    pub outcome: Outcome,
}

#[derive(Clone, Copy, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Direction {
    Up,
    Down,
}

#[derive(Clone, Copy, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Outcome {
    Correct,
    Wrong,
}

impl From<prices::LatestPrice> for PriceChanged {
    fn from(latest: prices::LatestPrice) -> Self {
        Self {
            price: latest.price().map(prices::Price::cents),
            delayed: latest.is_delayed(),
        }
    }
}
