# Take BTC/USD prices from Bitstamp, the only exchange whose terms allow showing them to Players

The server takes its BTC/USD prices from Bitstamp's public WebSocket feed, channel `live_trades_btcusd` on `wss://ws.bitstamp.net`, with no key.
Coinbase and Kraken trade more often, but their terms forbid showing their data to anyone outside your own organization without written consent.
Bitstamp's API page explicitly allows redistributing its exchange data and asks for a Data License Agreement only for commercial use.
Hoof & Claw is a non-commercial demo for a recruiting process with no plans to launch, so it needs no agreement with Bitstamp.

## Comparison

All terms were read on 2026-10-31.

| Source | Terms | What the terms allow | Price changes in a sample | Path to wider use |
| --- | --- | --- | --- | --- |
| Bitstamp | [API page](https://www.bitstamp.net/api/), section "Commercial Use of Bitstamp's Exchange Data" | "Bitstamp allows the incorporation and redistribution of our exchange data for commercial purposes"; companies using it "for their own commercial purposes" sign a Data License Agreement | 20 in 120 s, longest gap 23.7 s (Saturday 2026-10-31); 6 in 60 s, longest gap 31.5 s (weekend sample in the price source research) | Commercial use: sign the Data License Agreement via partners@bitstamp.net |
| Coinbase Exchange | [Market Data Terms of Use](https://www.coinbase.com/legal/market_data), last updated 2026-08-07 | Use only for "personal or research purposes"; it "may not be used to build an application intended for use by end users other than for you or your officers/employees"; no display "to any third party outside of your organization" without written consent; no use for developing, validating or operating AI or automated systems | 57 in 60 s, longest gap 9.9 s (price source research) | Written consent or a data agreement via marketdata@coinbase.com; Coinbase routes redistribution through [authorized partners](https://www.coinbase.com/institutional/market-data) |
| Kraken | [Global Terms of Service](https://www.kraken.com/legal/global-terms), last updated 2026-10-28; the EEA terms say the same | No market data terms of their own, but the general terms bind anyone "accessing or using our services or Platforms"; content may be used "only for your own benefit"; no making it "available to any third party", and no "third-party applications that interact with Our Content without our prior written consent" | 17 in 60 s, longest gap 8.1 s (price source research) | Written permission via marketdata@kraken.com, which the terms name for any other use |

## Considered Options

- **Coinbase Exchange, read as personal use.** Rejected.
  It has the most price changes, and the demo is personal in purpose.
  But reviewers who open the demo are end users outside our organization, and the terms forbid both the application and the display without written consent.
- **Kraken.** Rejected.
  The price source research found no market data terms and took that as no restriction.
  Kraken's general terms apply to the public API anyway, and read literally they forbid any third-party application.
- **Asking Coinbase or Kraken for consent first.** Not needed for the demo.
  It stays the path to a second source and to wider use, see below.
- **CoinGecko.** Rejected in the price source research.
  Its terms allow display with "Powered by CoinGecko", but the free plan refreshes at most once a minute and allows 10,000 calls a month.
- **Binance.** Rejected in the price source research.
  Binance.com quotes BTC against USDT, not USD.

## Consequences

- Guesses resolve later than they would on Coinbase or Kraken.
  With gaps of up to about 30 seconds between price changes, a guess usually resolves between 60 and 90 seconds after it was placed.
  The rules still hold, because a guess resolves on the first changed price after 60 seconds (ADR 0002).
- Bitstamp's tick size for BTC/USD is 0.01 USD, the same as Coinbase's, so the price precision in ADR 0002 stands.
- There is no fallback whose terms allow the demo.
  When Bitstamp's feed is down, nothing resolves and the price shown is stale; ADR 0007 decides how the game shows that and what happens to open guesses.
- The terms can change without notice, as Coinbase's did in August 2026.
  The spec names the terms read here, and they are read again before any step on the path below.
- Agent-run tests and test environments use a stubbed price source, not a live feed, which also keeps clear of Coinbase's clause on AI and automated systems should Coinbase become a source later.

## Path to wider use

The number of Players does not change what the server takes from the Price source, because one process holds one feed at every scale (ADR 0003).
What triggers outreach is how the demo is used and how reliable it must be, not traffic.

- **Commercial use of any kind**, such as ads, paid features or running it as a product: sign Bitstamp's Data License Agreement before launch.
- **A second source for failover or faster resolution**: agree terms with Coinbase (marketdata@coinbase.com or one of its authorized partners) or Kraken (marketdata@kraken.com) before connecting to it.
  Coinbase and Kraken changed price two to three times as often as Bitstamp in the research's samples, so either would also shorten the time a guess stays open.
  A second source also needs the rule from ADR 0002 that a guess only resolves against prices from the source of its Start price.
- **Moving to AWS at around 10,000 Players** (ADR 0003): re-read the terms of every source in use, and confirm the agreements cover the new deployment.
