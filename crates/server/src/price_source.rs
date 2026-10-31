//! The Price source of a release build until it takes prices from Bitstamp.

use prices::{PriceSource, Report};

/// A Price source that never connects, so the price stays Price delayed with no price.
#[cfg_attr(
    feature = "test-controls",
    expect(
        dead_code,
        reason = "a build with the test controls runs on the controllable source"
    )
)]
pub(crate) struct Unconnected;

impl PriceSource for Unconnected {
    async fn next(&mut self) -> Option<Report> {
        std::future::pending().await
    }
}
