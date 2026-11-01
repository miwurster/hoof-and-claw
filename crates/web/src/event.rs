//! The events of the stream and the snapshot it opens with (ADR 0014).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Every event on the stream, told apart by `type`.
#[derive(Clone, Debug, Serialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export)]
pub enum ServerEvent {
    Snapshot(Snapshot),
    Price(PriceChanged),
    GuessPlaced(GuessPlaced),
    GuessResolved(GuessResolved),
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

/// The Player placed a Guess, in this tab or another.
#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GuessPlaced {
    pub guess: OpenGuess,
}

/// The Player's Guess resolved, with their Score after it.
#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GuessResolved {
    pub guess: ResolvedGuess,
    #[ts(type = "number")]
    pub score: i64,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
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

impl From<prices::ShownPrice> for PriceChanged {
    fn from(latest: prices::ShownPrice) -> Self {
        Self {
            price: latest.price().map(prices::Price::cents),
            delayed: latest.is_delayed(),
        }
    }
}

impl From<guessing::Direction> for Direction {
    fn from(direction: guessing::Direction) -> Self {
        match direction {
            guessing::Direction::Up => Self::Up,
            guessing::Direction::Down => Self::Down,
        }
    }
}

impl From<Direction> for guessing::Direction {
    fn from(direction: Direction) -> Self {
        match direction {
            Direction::Up => Self::Up,
            Direction::Down => Self::Down,
        }
    }
}

impl From<game::Open> for OpenGuess {
    fn from(open: game::Open) -> Self {
        Self {
            id: open.guess.id.to_string(),
            direction: open.guess.direction.into(),
            start_price: open.guess.start.price.cents(),
            // Never more than 60 000.
            remaining_ms: u64::try_from(open.remaining.as_millis()).unwrap_or(u64::MAX),
        }
    }
}

impl From<guessing::Outcome> for Outcome {
    fn from(outcome: guessing::Outcome) -> Self {
        match outcome {
            guessing::Outcome::Correct => Self::Correct,
            guessing::Outcome::Wrong => Self::Wrong,
        }
    }
}

impl From<guessing::ResolvedGuess> for ResolvedGuess {
    fn from(resolved: guessing::ResolvedGuess) -> Self {
        Self {
            id: resolved.guess.id.to_string(),
            direction: resolved.guess.direction.into(),
            start_price: resolved.guess.start.price.cents(),
            end_price: resolved.end.price.cents(),
            outcome: resolved.outcome.into(),
        }
    }
}

impl From<game::PlayerEvent> for ServerEvent {
    fn from(event: game::PlayerEvent) -> Self {
        match event {
            game::PlayerEvent::GuessPlaced(open) => {
                Self::GuessPlaced(GuessPlaced { guess: open.into() })
            }
            game::PlayerEvent::GuessResolved(resolution) => Self::GuessResolved(GuessResolved {
                guess: resolution.guess.into(),
                score: resolution.score.value(),
            }),
        }
    }
}

impl From<game::Event> for ServerEvent {
    fn from(event: game::Event) -> Self {
        match event {
            game::Event::Price(latest) => Self::Price(latest.into()),
            game::Event::Player(event) => event.into(),
        }
    }
}
