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
