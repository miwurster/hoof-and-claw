use jiff::Timestamp;

use crate::{Price, TradeId};

/// The Price source a price came from (ADR 0004: Bitstamp is the only one the demo may use).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Source {
    Bitstamp,
}

/// A price as the server received it: from which Price source, and when by the server's clock (ADR 0002).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReceivedPrice {
    pub price: Price,
    pub source: Source,
    /// The trade the price came from, or `None` for a price that is no trade, such as the REST seed.
    pub trade_id: Option<TradeId>,
    pub received_at: Timestamp,
}
