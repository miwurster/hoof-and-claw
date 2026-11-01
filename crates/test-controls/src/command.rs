//! The wire types of the test controls' commands, exported to `contract/test-controls`.

use jiff::Timestamp;
use serde::Deserialize;
use ts_rs::TS;

/// The Price source sends a price, in cents.
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export, export_to = "test-controls/")]
pub struct SendPrice {
    #[ts(type = "number")]
    pub price: u64,
}

/// Restores the connection to the Price source, or loses it.
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export, export_to = "test-controls/")]
pub struct SetConnection {
    pub connected: bool,
}

/// Moves the clock forward.
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export, export_to = "test-controls/")]
pub struct AdvanceClock {
    #[ts(type = "number")]
    pub milliseconds: u64,
}

/// Puts the clock at a time, an RFC 3339 timestamp.
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export, export_to = "test-controls/")]
pub struct SetClock {
    #[ts(type = "string")]
    pub time: Timestamp,
}

/// Makes the store fail, as one state that replaces the one before.
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export, export_to = "test-controls/")]
pub struct SetStore {
    pub fails: StoreFailure,
}

/// What the store fails: nothing, every save of resolutions, or every operation.
#[derive(Clone, Copy, Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "test-controls/")]
pub enum StoreFailure {
    Nothing,
    Resolutions,
    Everything,
}
