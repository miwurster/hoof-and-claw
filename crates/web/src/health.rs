use serde::Serialize;
use ts_rs::TS;

/// The answer of `GET /health`, given once migrations ran and whether or not the price is delayed.
#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Health {
    build: String,
    test_controls: bool,
}

impl Health {
    #[must_use]
    pub fn new(build: String, test_controls: bool) -> Self {
        Self {
            build,
            test_controls,
        }
    }
}
