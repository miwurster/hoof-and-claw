use crate::{ReceivedPrice, ShownPrice};

/// The latest price the server received, with when and from which trade, and whether it is Price delayed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LatestPrice {
    received: Option<ReceivedPrice>,
    delayed: bool,
}

impl Default for LatestPrice {
    /// The start state: no price yet, and Price delayed until the Price source is live (ADR 0007).
    fn default() -> Self {
        Self {
            received: None,
            delayed: true,
        }
    }
}

impl LatestPrice {
    /// The price a Guess placed now starts from, or `None` while the price is delayed or none was received yet
    /// (ADR 0007).
    #[must_use]
    pub const fn live(self) -> Option<ReceivedPrice> {
        if self.delayed { None } else { self.received }
    }

    /// What a stream shows of it.
    #[must_use]
    pub const fn shown(self) -> ShownPrice {
        let price = match self.received {
            Some(received) => Some(received.price),
            None => None,
        };
        ShownPrice::new(price, self.delayed)
    }

    /// Takes in a received price, which becomes the latest one.
    pub const fn receive(&mut self, received: ReceivedPrice) {
        self.received = Some(received);
    }

    /// Takes in a change between live and Price delayed; the last price stays either way.
    pub const fn set_delayed(&mut self, delayed: bool) {
        self.delayed = delayed;
    }
}

#[cfg(test)]
mod tests {
    use jiff::Timestamp;

    use super::*;
    use crate::{Price, Source};

    const RECEIVED: ReceivedPrice = ReceivedPrice {
        price: Price::from_cents(6_400_000),
        source: Source::Bitstamp,
        trade_id: None,
        received_at: Timestamp::constant(1_793_448_000, 0),
    };

    #[test]
    fn a_server_that_has_just_started_has_no_price_and_is_price_delayed() {
        let latest = LatestPrice::default();

        assert_eq!(latest.live(), None);
        assert_eq!(latest.shown(), ShownPrice::new(None, true));
    }

    #[test]
    fn a_received_price_becomes_the_live_price() {
        let mut latest = LatestPrice::default();

        latest.set_delayed(false);
        latest.receive(RECEIVED);

        assert_eq!(latest.live(), Some(RECEIVED));
        assert_eq!(latest.shown(), ShownPrice::new(Some(RECEIVED.price), false));
    }

    #[test]
    fn while_the_price_is_delayed_the_last_price_stays_shown_but_is_not_live() {
        let mut latest = LatestPrice::default();
        latest.set_delayed(false);
        latest.receive(RECEIVED);

        latest.set_delayed(true);

        assert_eq!(latest.live(), None);
        assert_eq!(latest.shown(), ShownPrice::new(Some(RECEIVED.price), true));
    }

    #[test]
    fn a_price_received_while_delayed_stays_delayed() {
        let mut latest = LatestPrice::default();

        latest.receive(RECEIVED);

        assert_eq!(latest.live(), None);
        assert_eq!(latest.shown(), ShownPrice::new(Some(RECEIVED.price), true));
    }

    #[test]
    fn two_trades_at_the_same_price_show_the_same() {
        let mut first = LatestPrice::default();
        first.set_delayed(false);
        first.receive(RECEIVED);
        let mut second = first;

        second.receive(ReceivedPrice {
            received_at: Timestamp::constant(1_793_448_001, 0),
            ..RECEIVED
        });

        assert_ne!(first, second);
        assert_eq!(first.shown(), second.shown());
    }
}
