//! Bitstamp's exact decimal prices, `price_str` and the ticker's `last`, read into cents.

use prices::Price;

/// A price Bitstamp sent that is not a whole number of cents.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum InvalidPrice {
    #[error("price {0:?} is not a decimal number of dollars")]
    NotADecimal(String),
    #[error("price {0:?} has more than two decimal places")]
    TooManyDecimalPlaces(String),
}

/// Reads a decimal such as `64000.12` into cents, exactly; never rounds (ADR 0009).
pub(crate) fn parse(text: &str) -> Result<Price, InvalidPrice> {
    let not_a_decimal = || InvalidPrice::NotADecimal(text.to_owned());
    let (dollars, fraction) = text.split_once('.').unwrap_or((text, "00"));
    let is_digits = |part: &str| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit());
    if !is_digits(dollars) || !is_digits(fraction) {
        return Err(not_a_decimal());
    }
    let scale = match fraction.len() {
        1 => 10,
        2 => 1,
        _ => return Err(InvalidPrice::TooManyDecimalPlaces(text.to_owned())),
    };
    let dollars: u64 = dollars.parse().map_err(|_| not_a_decimal())?;
    let cents = fraction.parse::<u64>().map_err(|_| not_a_decimal())? * scale;
    dollars
        .checked_mul(100)
        .and_then(|cents_of_dollars| cents_of_dollars.checked_add(cents))
        .map(Price::from_cents)
        .ok_or_else(not_a_decimal)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_price_with_two_decimal_places_is_read_into_cents() {
        assert_eq!(parse("64000.12"), Ok(Price::from_cents(6_400_012)));
    }

    #[test]
    fn a_price_with_fewer_decimal_places_is_read_into_cents() {
        assert_eq!(parse("64000.5"), Ok(Price::from_cents(6_400_050)));
        assert_eq!(parse("64000"), Ok(Price::from_cents(6_400_000)));
    }

    #[test]
    fn a_price_with_more_than_two_decimal_places_is_an_error() {
        assert_eq!(
            parse("64000.123"),
            Err(InvalidPrice::TooManyDecimalPlaces("64000.123".to_owned()))
        );
    }

    #[test]
    fn a_price_that_is_no_decimal_is_an_error() {
        for text in [
            "",
            ".",
            "1.",
            ".5",
            "-1.00",
            "1e3",
            "64,000.00",
            "1.2.3",
            "99999999999999999999",
        ] {
            assert_eq!(
                parse(text),
                Err(InvalidPrice::NotADecimal(text.to_owned())),
                "{text:?}"
            );
        }
    }
}
