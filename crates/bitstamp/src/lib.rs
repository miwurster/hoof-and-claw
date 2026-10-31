//! The `PriceSource` adapter for Bitstamp: the `live_trades_btcusd` WebSocket feed and the REST ticker as seed.

mod backoff;
mod message;
mod price;
mod price_source;
mod seed;

pub use price_source::{Bitstamp, Config, FEED_URL, Shutdown, StartError, TICKER_URL};
