# Stream every Bitstamp trade and refuse guesses while the price is delayed

The latest available price is the price of the last trade the server received on Bitstamp's `live_trades_btcusd` channel (ADR 0004).
The server processes every trade and does not sample.
After each connect it reads Bitstamp's REST ticker once, so it has a price before the next trade arrives.
While the connection to Bitstamp is not healthy, the price is Price delayed: the server refuses new guesses, and Open guesses wait until a changed price arrives, with no time limit.

## Bitstamp facts this rests on

Bitstamp's WebSocket docs page (`https://www.bitstamp.net/websocket/v2/`) blocks automated reads, so these facts come from its newest archived copy (Wayback Machine, 2025-05-22) and from a 90-second live session on 2026-10-31.

- A client checks that the server is responsive by sending `{"event": "bts:heartbeat"}`; the live server answered with `"status": "success"` within about 20 ms.
  The docs name no heartbeat interval and no idle timeout, and 22.7 s without traffic did not drop the connection.
- Before maintenance, the server sends `bts:request_reconnect`; the client then has "a few seconds" to reconnect, and a new connection continues "without any message loss".
- A connection lives at most 90 days, about as long as the demo.
- A trade event carries `price` as a JSON float and `price_str` as a string, plus the trade `id` and timestamps.
  The REST ticker `GET /api/v2/ticker/btcusd/` returns `last` as a string, with no trade id and a timestamp in whole seconds.
- The live session saw 12 trades in 90 s, with gaps between 1.5 s and 22.7 s; earlier samples saw gaps up to 31.5 s (ADR 0004).
- An hour from Render Frankfurt on 2026-10-31 saw 1,159 trades with gaps up to 39.3 s, and all 376 heartbeats answered within 43 ms on one connection that never dropped.

## Decision

- **Latest price.** The price of the last trade received, or the REST ticker's `last` after a connect.
  The server never uses the order book's mid price, bid or ask, or other ticker values, so the price shown, the Start price and the End price all mean the same thing (ADR 0002).
- **Exact prices.** The server reads `price_str` and `last` as exact decimals, never the float `price`.
- **No sampling.** Every trade updates the latest price and runs the resolution check over the index of due guesses.
  The cost per trade depends on the number of guesses that are due, not on the number of Players.
- **Price stream to the browser.** It carries price changes only; a trade at the same price sends nothing.
  Each tab holds one latest-value slot for the price, not a queue, so a client that cannot keep up skips intermediate prices and always gets the current one.
  Memory and bandwidth per client therefore do not grow with the trade rate.
  The server never drops a Player's resolution events, and they do not share that slot.
  On top of that, one server-wide throttle sends at most one price event a second (ADR 0010).
- **Seed on connect.** Right after subscribing, the server calls the REST ticker once and treats `last` as a received price like any trade.
  It can serve as a Start price, or as the End price of a guess that fell due during an outage, because it is the current last trade, not a backfilled one.
- **Healthy.** The connection counts as healthy while it is open, `bts:subscription_succeeded` has arrived, and every heartbeat, sent every 10 s, is answered within 5 s.
- **Price delayed.** Every other state is Price delayed: disconnected, not yet subscribed, or a heartbeat unanswered.
  A server that has just started is not yet subscribed, so it starts in Price delayed.
  There is no grace period, and one state drives both the server and the UI.
  The server rejects place-a-Guess commands, and the client shows the last price as "Price delayed" with Up and Down disabled (visual direction).
- **Silent subscription.** If no trade arrives for 90 s while heartbeats are answered, the server assumes the subscription hangs, reconnects and seeds again.
- **Planned reconnects.** On `bts:request_reconnect` and before the 90-day limit, the server opens and subscribes a new connection before it closes the old one, so a planned reconnect never shows Price delayed.
- **Unplanned reconnects.** The server retries after 1 s, doubles the wait up to 30 s, adds jitter, and never gives up.
- **Open guesses during an outage.** They stay open until a changed price from their Start price's source arrives, however long that takes; there is no void and no timeout.
- **What is stored.** Each guess stores its Start price and End price, each with `receivedAt`, the Price source and Bitstamp's trade id.
  A seeded price has no trade id, so that field stays empty.
  The server keeps no table of received prices and does not persist the latest price across restarts.

The heartbeat interval, reply timeout, silence limit and backoff bounds are configuration, with the values above as defaults.

## Considered Options

- **Polling the REST ticker on an interval.** Rejected.
  It spends request budget, delays resolution by up to one interval, and adds a sampling interval to tune, while the WebSocket feed pushes every trade for free.
- **Order book mid price or bid/ask as the latest price.** Rejected.
  The brief asks for the market price, and a trade is what the market actually paid; mixing meanings would break ADR 0002's one rule for Start and End price.
- **Waiting for the first trade instead of seeding.** Rejected.
  It would show Price delayed for up to about 30 s after every start or reconnect.
- **Marking the price delayed after a period without trades.** Rejected.
  Gaps of 20 to 30 s are normal on Bitstamp, so any threshold short enough to matter would flag a quiet market as an outage.
  Silence only triggers a reconnect, at 90 s.
- **A grace period before Price delayed.** Rejected.
  Planned reconnects already avoid the gap, and a guess against a price from a connection that is down is exactly what the rule exists to prevent.
- **Disabling the buttons in the UI only.** Rejected.
  A Player who watches another exchange during a Bitstamp outage could otherwise send a guess against a stale Start price and win with certainty.
- **Voiding an Open guess after some minutes without a price.** Rejected.
  It needs a new Outcome, state and UI, and with one source a Player whose guess is voided still could not place a new one, because guesses are refused while the price is delayed.
  A source that never comes back, for example because Bitstamp blocks Render's addresses, is an operations incident solved by changing the source, not a game rule.
- **Storing every received price.** Rejected.
  The audit trail an Outcome needs is its Start price and End price with time, source and trade id, which the guess already holds.
  A price history would add writes on every trade and a retention question without serving any rule of the game.
- **Persisting the latest price across restarts.** Rejected.
  After a restart that price is stale by definition, and the REST seed gives a fresh one within one request.

## Consequences

- A guess usually resolves 60 to 90 s after it was placed, as ADR 0004 expects.
- While Bitstamp is down, nobody can guess and nothing resolves; when it returns, every due guess resolves against the first changed price received.
- Bitstamp's archived docs may differ from the live page.
  The feed check from Render ("Check the price feeds from Render Frankfurt") confirmed the heartbeat behaviour, but saw no `bts:request_reconnect` in its hour, so planned reconnects still rest on the archived docs.
- Alerting when the feed stays delayed belongs to observability, not to this decision.
