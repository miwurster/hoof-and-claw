# BTC/USD price sources with a usable free tier

Research for [#2](https://github.com/miwurster/hoof-and-claw/issues/2), part of the map [#1](https://github.com/miwurster/hoof-and-claw/issues/1).
All sources were read or probed on 2026-10-31.

## Question

Which third-party sources can give the server a BTC/USD price fairly and often enough to resolve Guesses, for free?
For Coinbase, Kraken, Bitstamp, Binance, CoinGecko and CoinCap: rate limits without and with a free key, REST against WebSocket, update frequency, region or cloud IP blocking, and whether the terms allow showing the price in a public app.

## Answer

Use the public **Coinbase Exchange** WebSocket feed (`matches` channel, `BTC-USD`) as the primary source and the **Kraken** WebSocket v2 `trade` channel (`BTC/USD`) as the fallback.
Both need no key, publish every trade in real USD, and change price several times a minute even in quiet hours.
Bitstamp works the same way but trades too rarely to be the primary source.
Binance.com quotes USDT, not USD, and user reports say it blocks US and cloud IPs.
Binance.US trades too rarely.
CoinGecko updates at most once a minute on the free tier and requires attribution, which is too slow for the End price rule.
CoinCap's keyless API no longer exists.

## Legend

- **Verified**: read on the owner's page or observed by a request on 2026-10-31.
- **Inferred**: my conclusion from verified facts, not stated by the owner.
- **Unverified**: the owner's page could not be read; the claim rests on secondary reports.

## Comparison

| Source | Keyless REST limit | Free key | WebSocket | Pair | Trades per minute (sample) | Blocking | Terms for public display |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Coinbase Exchange | 10 req/s per IP, burst 15 | not needed | yes, no auth | BTC-USD | 189 trades, 57 price changes | none documented | Market Data Terms exist, not readable (403) |
| Kraken | about 1 call/s per IP | not needed | yes, no auth | BTC/USD | 21 trades, 17 price changes | none documented | no API terms found |
| Bitstamp | 400 req/s, 10,000 per 10 min | not needed | yes, no auth | BTC/USD | 7 trades, 6 price changes | none documented | commercial use needs a Data License Agreement |
| Binance.com | weight-based per IP | not needed | yes, no auth | BTC/USDT only | 811 trades, 191 price changes | US and many cloud IPs (secondary reports) | not readable |
| Binance.US | weight-based per IP | not needed | yes, no auth | BTC/USD | 4 trades, 3 price changes | US-only exchange | not checked |
| CoinGecko | shared per IP, no number given | Demo: 100 calls/min, 10,000 calls/month | no | aggregated | data "from 60 sec" | none documented | "Powered by CoinGecko" required |
| CoinCap | none (v2 host gone) | v3 needs a key | not checked | aggregated | n/a | n/a | not checked |

The sample column comes from one 60-second WebSocket capture per feed, run in parallel on Saturday 2026-10-31 08:42 to 08:43 UTC from a residential IP.
A weekend morning is a quiet period, so these counts are a lower bound, not an average.

## Per source

### Coinbase Exchange

- Verified: public REST endpoints allow 10 requests per second per IP, with bursts up to 15, and answer `429` above that ([Coinbase Exchange REST rate limits](https://docs.cdp.coinbase.com/exchange/rest-api/rate-limits)).
- Verified: the market data WebSocket `wss://ws-feed.exchange.coinbase.com` is "available without authentication"; a client must subscribe within 5 seconds of connecting ([WebSocket overview](https://docs.cdp.coinbase.com/exchange/websocket-feed/overview)).
- Verified: WebSocket limits are 8 connection requests per second per IP (burst 20) and 100 client messages per second per IP ([WebSocket rate limits](https://docs.cdp.coinbase.com/exchange/websocket-feed/rate-limits)).
- Verified: `GET https://api.exchange.coinbase.com/products/BTC-USD/ticker` without a key returned the last trade price, its `trade_id` and its trade `time`, with `cache-control: public, max-age=1`.
- Verified: the `matches` channel delivered 189 trades and 57 distinct prices in 60 seconds; the longest gap between price changes was 9.9 seconds.
- Verified: `GET https://api.coinbase.com/v2/prices/BTC-USD/spot` (the retail Coinbase app API) also works keyless, but returns a derived price without a trade id or timestamp, so it is a worse fit for an auditable End price.
- Unverified: Coinbase publishes [Market Data Terms of Use](https://www.coinbase.com/legal/market_data), last updated 2023-02-06 according to a search snippet; the page returned `403` to both WebFetch and curl, so its display and redistribution clauses are unread.
- Unverified: Coinbase's [institutional market data page](https://www.coinbase.com/institutional/market-data) says redistribution goes through authorized partners.
- Inferred: no page I read mentions blocking regions or cloud IP ranges for public market data.

### Kraken

- Verified: Kraken's support article says public market data endpoints are limited per IP, and staying at or below one call per second avoids the limit ([Kraken API rate limits](https://support.kraken.com/articles/206548367-what-are-the-api-rate-limits-)).
- Verified: the [REST rate limit guide](https://docs.kraken.com/api/docs/guides/spot-rest-ratelimits/) describes only per-key counters and gives no keyless number.
- Verified: WebSocket v2 at `wss://ws.kraken.com/v2` offers a `ticker` channel whose default `event_trigger` is `trades`, publishing on every trade, and whose `last` field is the last traded price ([WebSocket v2 ticker](https://docs.kraken.com/api/docs/websocket-v2/ticker)).
- Verified: the `trade` channel for `BTC/USD` worked without auth and delivered 21 trades and 17 distinct prices in 60 seconds; the longest gap was 8.1 seconds.
- Verified: `GET https://api.kraken.com/0/public/Ticker?pair=XBTUSD` worked keyless with `cache-control: public, max-age=2`.
- Verified: the [Kraken legal page](https://www.kraken.com/legal) links no API or market data terms; `https://www.kraken.com/legal/api-terms` returns `404`.
- Inferred: with no published data terms, displaying Kraken's price in a free demo carries no stated restriction, but also no stated permission.

### Bitstamp

- Verified: "all clients can make 400 requests per second", with a default of 10,000 requests per 10 minutes ([Bitstamp API](https://www.bitstamp.net/api/)).
- Verified: the same page says Bitstamp "allows the incorporation and redistribution of our exchange data for commercial purposes" and directs companies to sign a commercial use Data License Agreement via partners@bitstamp.net.
- Verified: `GET https://www.bitstamp.net/api/v2/ticker/btcusd/` worked keyless with `cache-control: max-age=1`.
- Verified: the `live_trades_btcusd` channel on `wss://ws.bitstamp.net` worked without auth, but delivered only 7 trades in 60 seconds, with a 31.5-second gap between price changes.
- Inferred: Bitstamp is the source with the clearest written data policy, but its low trade rate makes Guesses stay open longer than on Coinbase or Kraken.

### Binance

- Verified: Binance limits are per IP, not per key, measured in request weight; exceeding them gives `429`, and ignoring `429` leads to `418` IP bans from 2 minutes to 3 days ([Binance spot limits](https://developers.binance.com/docs/binance-spot-api-docs/rest-api/limits)).
- Verified: Binance recommends `https://data-api.binance.vision` for public market data only ([General API information](https://developers.binance.com/docs/binance-spot-api-docs/rest-api/general-api-information)).
- Verified: Binance.com lists BTC against USDT, not USD; at 08:40 UTC its `BTCUSDT` price was about 150 USD above the USD prices on Coinbase, Kraken and Bitstamp.
- Verified: Binance.US (`api.binance.us`, `BTCUSD`) worked keyless but delivered only 4 trades in 60 seconds.
- Unverified: user reports on [CCXT](https://github.com/ccxt/ccxt/issues/15917), [Linode](https://www.linode.com/community/questions/23972/linode-and-binancecom), [Railway](https://station.railway.com/questions/deployment-is-crashing-7b190d57) and [Vercel](https://community.vercel.com/t/binance-api-calls-on-vercel/29895) describe `451` responses from Binance.com citing the "Eligibility" section of its terms, for US IPs and for several cloud providers' regions.
- Inferred: Binance.com does not give a BTC/USD price at all, and its cloud blocking makes it a bad fit for an always-on free-tier server.

### CoinGecko

- Verified: the Demo plan (free key) allows 100 calls per minute and 10,000 calls per month, lists data freshness "from 60 sec", and requires attribution ([CoinGecko API pricing](https://www.coingecko.com/en/api/pricing)).
- Verified: the keyless public API is rate limited per IP and "shared across all users on the same IP", with no number given ([Common errors and rate limit](https://docs.coingecko.com/docs/common-errors-rate-limit)).
- Verified: the [API Terms](https://www.coingecko.com/en/api_terms), version of 2025-09-05, require displaying "Powered by CoinGecko" prominently and say they "do not encourage caching or storage of Data".
- Verified: `/api/v3/simple/price?ids=bitcoin&vs_currencies=usd` returned `cache-control: max-age=30, s-maxage=60` and a `last_updated_at` about 30 seconds old.
- Inferred: 10,000 calls a month is one call every 4.5 minutes when spread evenly, far below what a one-minute game needs.
- Inferred: an aggregated, cached price that moves at most once a minute stretches every Guess to two minutes or more under the End price rule.

### CoinCap

- Verified: `api.coincap.io` (the keyless v2 API) no longer resolves; the request failed to connect.
- Verified: `https://rest.coincap.io/v3/assets/bitcoin` without a key returns `401 Unauthorized`.
- Unverified: a [CoinPaprika migration guide](https://coinpaprika.com/education/migrating-from-coincap-api-to-coinpaprika/), from a competitor, says CoinCap moved to an account- and credit-based model in 2025.
- Verified: [pro.coincap.io/api-docs](https://pro.coincap.io/api-docs) and its pricing page render client-side and gave no readable content, so the free credit allowance is unknown.

## Fit for a fair End price

The End price is the first price at least 60 seconds after the Guess that differs from the Start price (`GLOSSARY.md`).
That rule needs a source that reports individual trades, with a timestamp from the exchange, in real USD.

- Verified: Coinbase (`matches`, ticker `trade_id` and `time`), Kraken (`trade`, ticker `last`) and Bitstamp (`live_trades`) all publish trade-level prices with exchange timestamps.
- Inferred: CoinGecko and the Coinbase retail spot price are aggregates without trade ids, so the server could not later show which trade set an End price.
- Inferred: a last-trade price from one exchange can be moved by a single small trade; at demo scale this does not matter, but it belongs in the path to large scale.

## Hosting implications

- Inferred: all recommended sources push trades over a long-lived WebSocket, which fits the map's "always-on server" and "a guess resolves even when no browser is open".
- Inferred: polling REST once per second stays inside Coinbase's and Kraken's keyless limits and is a workable fallback if a host forbids long-lived outbound connections.
- Not verified: none of the sources were probed from a cloud provider IP; only Binance has reports of cloud blocking.

## Open questions for a later decision

1. Coinbase and Kraken differed by about 5 USD at the same moment, so failing over from one to the other mid-guess can fake a price change.
   Should a Guess stay pinned to the source that gave its Start price, and what happens to an Open guess when that source is down?
2. Coinbase's Market Data Terms of Use returned `403` to automated reads.
   Someone has to read them in a browser before the ADR picks Coinbase, or the ADR picks Kraken or Bitstamp instead.
3. Nobody has called Coinbase or Kraken from the chosen hosting platform's egress IPs yet; the hosting ADR should include that check.
4. How long without a trade counts as a dead feed, given observed gaps of 10 to 30 seconds between price changes?
