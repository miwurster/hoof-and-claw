# The End price is the first changed price the server receives after 60 seconds

A Guess resolves against the first price the server receives at least 60 seconds after it accepted the Guess (`receivedAt >= placedAt + 60 s`) that differs from the Start price by at least one tick and comes from the same Price source as the Start price.
The Start price is the last price the server received before it accepted the Guess.
Both times come from the server's clock, because the server is the only authority on prices and outcomes and one clock leaves no skew at the 60-second boundary.

## Considered Options

- **The exchange's trade timestamp.** Rejected.
  It is stricter at the boundary, since a trade at 59.9 s that arrives at 60.1 s would not count, but it compares the exchange's clock with ours, and clock skew shifts the boundary.
- **The resolver's processing time.** Rejected.
  It depends on how busy the server is.
- **After an outage, the historical trade just past 60 seconds, backfilled over REST.** Rejected.
  It needs a backfill adapter and a second, historical meaning of "observed".
  The Player cannot act while a Guess is open, so resolving against the first price after recovery gives nobody an advantage.
- **A minimum move, such as 1 USD.** Rejected.
  The brief says the Guess resolves "when the price changes", so any difference of one tick counts.
- **Fetching a fresh Start price when a Guess arrives.** Rejected.
  It costs a call to the Price source per Guess, and the Start price would no longer follow the same rule as the End price.

## Consequences

- The server holds prices exactly at the source's tick size, 0.01 USD for Bitstamp BTC/USD (ADR 0004), as an integer of the smallest unit or an exact decimal, never as floating point.
- A price equal to the Start price does not count, and the Guess stays open.
- A Guess records the Price source of its Start price and ignores prices from any other source, because Coinbase and Kraken differ by a few dollars and switching sources mid-guess would fake a change.
  If that source never recovers, the Guess stays open; ADR 0007 decides that it stays open with no void or timeout.
- A Guess that falls due during an outage or a restart resolves against the first changed price received afterwards, so the server must persist a Guess before it acknowledges it.
