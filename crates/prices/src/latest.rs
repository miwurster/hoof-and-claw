use crate::{Price, Report};

/// The latest price the server received, and whether it is Price delayed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LatestPrice {
    price: Option<Price>,
    delayed: bool,
}

impl Default for LatestPrice {
    /// The start state: no price yet, and Price delayed until the Price source is live (ADR 0007).
    fn default() -> Self {
        Self {
            price: None,
            delayed: true,
        }
    }
}

impl LatestPrice {
    /// The last price received, or `None` while no price has been received since the start.
    #[must_use]
    pub const fn price(self) -> Option<Price> {
        self.price
    }

    #[must_use]
    pub const fn is_delayed(self) -> bool {
        self.delayed
    }

    /// Takes in what the Price source reported.
    pub const fn apply(&mut self, report: Report) {
        match report {
            Report::Received(price, _) => self.price = Some(price),
            Report::Live => self.delayed = false,
            Report::Delayed => self.delayed = true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PRICE: Price = Price::from_cents(6_400_000);

    #[test]
    fn a_server_that_has_just_started_has_no_price_and_is_price_delayed() {
        let latest = LatestPrice::default();

        assert_eq!(latest.price(), None);
        assert!(latest.is_delayed());
    }

    #[test]
    fn a_received_price_becomes_the_latest_price() {
        let mut latest = LatestPrice::default();

        latest.apply(Report::Live);
        latest.apply(Report::Received(PRICE, None));

        assert_eq!(latest.price(), Some(PRICE));
        assert!(!latest.is_delayed());
    }

    #[test]
    fn while_the_price_is_delayed_the_last_price_stays() {
        let mut latest = LatestPrice::default();
        latest.apply(Report::Live);
        latest.apply(Report::Received(PRICE, None));

        latest.apply(Report::Delayed);

        assert_eq!(latest.price(), Some(PRICE));
        assert!(latest.is_delayed());
    }

    #[test]
    fn a_price_received_while_delayed_stays_delayed() {
        let mut latest = LatestPrice::default();

        latest.apply(Report::Received(PRICE, None));

        assert_eq!(latest.price(), Some(PRICE));
        assert!(latest.is_delayed());
    }
}
