use crate::Price;

/// What a stream shows of the price: the latest price, and whether it is Price delayed.
///
/// Two values are equal when a stream would show the same, whenever and from which trade the price was received.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShownPrice {
    price: Option<Price>,
    delayed: bool,
}

impl Default for ShownPrice {
    /// The start state: no price yet, and Price delayed until the Price source is live (ADR 0007).
    fn default() -> Self {
        Self::new(None, true)
    }
}

impl ShownPrice {
    #[must_use]
    pub const fn new(price: Option<Price>, delayed: bool) -> Self {
        Self { price, delayed }
    }

    /// The last price received, or `None` while no price has been received since the start.
    #[must_use]
    pub const fn price(self) -> Option<Price> {
        self.price
    }

    #[must_use]
    pub const fn is_delayed(self) -> bool {
        self.delayed
    }
}
