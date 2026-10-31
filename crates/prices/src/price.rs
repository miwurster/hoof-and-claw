/// The ID the Price source gave the trade a price came from; Bitstamp numbers its trades.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TradeId(u64);

impl TradeId {
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}

/// A BTC/USD price as a whole number of cents, Bitstamp's tick (ADR 0009).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Price(u64);

impl Price {
    #[must_use]
    pub const fn from_cents(cents: u64) -> Self {
        Self(cents)
    }

    #[must_use]
    pub const fn cents(self) -> u64 {
        self.0
    }
}
