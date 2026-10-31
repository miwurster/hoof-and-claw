//! The `PriceSource` port: what the Price source reports, one report at a time.

use crate::Price;

/// One thing the Price source reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Report {
    /// The server received a price.
    Received(Price),
    /// The connection to the Price source became healthy.
    Live,
    /// The connection to the Price source is no longer healthy: Price delayed.
    Delayed,
}

/// Delivers what the Price source reports, in the order it happened.
pub trait PriceSource: Send + 'static {
    /// Waits for the next report, or `None` once the source has ended.
    ///
    /// The future must be cancel-safe: dropping it before it completes loses no report.
    fn next(&mut self) -> impl Future<Output = Option<Report>> + Send;
}

/// The shared test suite every `PriceSource` adapter passes, each through its own driver.
#[expect(
    clippy::missing_panics_doc,
    reason = "a test suite fails by panicking, as every test does"
)]
pub mod suite {
    use super::{PriceSource, Report};
    use crate::Price;

    /// Makes the Price source behind an adapter do something, the way that adapter's tests can.
    pub trait Driver {
        fn sends(&self, price: Price) -> impl Future<Output = ()>;
        fn loses_the_connection(&self) -> impl Future<Output = ()>;
        fn restores_the_connection(&self) -> impl Future<Output = ()>;
    }

    pub async fn a_price_the_source_sends_is_received(
        mut source: impl PriceSource,
        driver: impl Driver,
    ) {
        driver.restores_the_connection().await;
        driver.sends(Price::from_cents(6_400_000)).await;

        assert_eq!(source.next().await, Some(Report::Live));
        assert_eq!(
            source.next().await,
            Some(Report::Received(Price::from_cents(6_400_000)))
        );
    }

    pub async fn losing_the_connection_is_price_delayed_until_it_is_restored(
        mut source: impl PriceSource,
        driver: impl Driver,
    ) {
        driver.restores_the_connection().await;
        driver.loses_the_connection().await;
        driver.restores_the_connection().await;

        assert_eq!(source.next().await, Some(Report::Live));
        assert_eq!(source.next().await, Some(Report::Delayed));
        assert_eq!(source.next().await, Some(Report::Live));
    }
}
