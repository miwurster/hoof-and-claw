//! What Bitstamp's WebSocket feed sends and what the adapter sends it.

use serde_json::{Value, json};

use crate::price::{self, InvalidPrice};
use prices::{Price, TradeId};

/// The only channel the adapter subscribes to (ADR 0004).
const CHANNEL: &str = "live_trades_btcusd";

/// One message from Bitstamp, as far as the adapter cares.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Message {
    SubscriptionSucceeded,
    /// A trade's price, with its `id`; a trade without a numeric `id` still counts, with none.
    Trade(Price, Option<TradeId>),
    HeartbeatAnswered,
    RequestReconnect,
    /// The subscription ended, or Bitstamp refused a request.
    Unsubscribed,
    Other,
}

pub(crate) fn subscribe() -> String {
    json!({ "event": "bts:subscribe", "data": { "channel": CHANNEL } }).to_string()
}

pub(crate) fn heartbeat() -> String {
    json!({ "event": "bts:heartbeat" }).to_string()
}

/// Reads one text frame; a trade whose `price_str` is not a whole number of cents is an error.
pub(crate) fn parse(text: &str) -> Result<Message, InvalidPrice> {
    let Ok(message) = serde_json::from_str::<Value>(text) else {
        return Ok(Message::Other);
    };
    let text_at = |pointer: &str| message.pointer(pointer).and_then(Value::as_str);
    let on_channel = text_at("/channel") == Some(CHANNEL);
    Ok(match text_at("/event") {
        Some("trade") if on_channel => {
            let price_str = text_at("/data/price_str").unwrap_or_default();
            let id = message
                .pointer("/data/id")
                .and_then(Value::as_u64)
                .map(TradeId::new);
            Message::Trade(price::parse(price_str)?, id)
        }
        Some("bts:subscription_succeeded") if on_channel => Message::SubscriptionSucceeded,
        Some("bts:heartbeat") => Message::HeartbeatAnswered,
        Some("bts:request_reconnect") => Message::RequestReconnect,
        Some("bts:unsubscription_succeeded" | "bts:error") => Message::Unsubscribed,
        _ => Message::Other,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_trade_is_read_from_its_exact_price_str_never_the_float_price() {
        let text = r#"{"event":"trade","channel":"live_trades_btcusd","data":{"id":1,"price":64000.1,"price_str":"64000.12"}}"#;

        assert_eq!(
            parse(text),
            Ok(Message::Trade(
                Price::from_cents(6_400_012),
                Some(TradeId::new(1))
            ))
        );
    }

    #[test]
    fn a_trade_with_more_than_two_decimal_places_is_an_error() {
        let text = r#"{"event":"trade","channel":"live_trades_btcusd","data":{"id":1,"price":1.0,"price_str":"64000.123"}}"#;

        assert!(parse(text).is_err());
    }
}
