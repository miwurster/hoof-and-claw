# Prices as integer cents

The server holds every BTC/USD price as a newtype over an unsigned integer number of cents, Bitstamp's tick of 0.01 USD (ADR 0004).
ADR 0002 left the choice between an integer of the smallest unit and an exact decimal open; this settles it for the integer, because comparing a price with the Start price is the only arithmetic the game does, and an integer does it exactly with no extra crate.
The `bitstamp` adapter parses `price_str` and the ticker's `last` (ADR 0007) straight into cents and rejects a price with more than two decimal places as an error, never rounding it.

## Considered Options

- **A decimal crate such as `rust_decimal`.** Rejected.
  It is exact too, but it adds a dependency and arithmetic the game never needs.
- **Floating point.** Rejected in ADR 0002, and Clippy forbids `f32` and `f64` in the cores (ADR 0008).

## Consequences

- A second Price source with a finer tick needs a wider unit, which changes the newtype and the stored column.
