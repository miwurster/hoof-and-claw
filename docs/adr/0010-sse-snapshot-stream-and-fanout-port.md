# One SSE stream per tab that opens with a snapshot, fed through a `Fanout` port

Each tab opens one server-sent events (SSE) stream, and its first event is a snapshot of everything the tab shows.
After that the stream carries price changes, at most one a second for the whole server, and every event about the tab's Player.
The `game` crate resolves Open guesses on every trade it receives, saves the resolutions before it publishes them, and hands all events to a `Fanout` port.
At demo scale the port has an in-process adapter, and its seam is where prices and Player events leave the process on the scale path.
This settles what ADR 0003, ADR 0005 and ADR 0007 left open about real-time delivery; the research behind it is "[Real-time patterns for price fan-out and guess resolution at three scales](../research/real-time-patterns.md)".

## Decision

- **Snapshot first.** The first event on every connection is a snapshot.
  It holds the latest price, whether the price is live or Price delayed, the Player's Score, and the Open guess with its Direction, Start price, remaining time and the server's current time.
  The client builds its whole state from the snapshot on every connect and reconnect.
  There is no replay through `Last-Event-ID`.
  The HTTP read of the Player's state stays for uses outside the stream.
- **Idempotent events.** The server subscribes the tab before it builds the snapshot, so an event from that short window may arrive after a snapshot that already contains it.
  Events therefore carry absolute values, such as the new Score and the guess's ID, not deltas.
  The client's reducer ignores a resolution for a guess it does not hold as open, so applying an event twice changes nothing.
- **Price throttle.** `game` publishes at most one price event a second for the whole server, and only when the price changed.
  The first change after a quiet second goes out at once.
  A later change within the same second goes out when the second ends, with the newest price, so the stream never lags the latest price by more than one second.
  Changes between live and Price delayed are not throttled.
  Resolution still uses every trade (ADR 0007).
  The throttle sits in `game`, before the port, so every adapter gets the same rate.
  It runs on `tokio`'s real time, not on the `Clock` port, which is the domain's time only (ADR 0011).
- **Every tab of the Player.** Every event about a Player goes to every open tab of that Player: Guess placed, and Guess resolved with its Outcome, End price and new Score.
  A guess placed in one tab therefore shows up in the others.
- **Order on each trade.** For each received price, `game`:
  1. takes from the index every Open guess that is due and whose Start price differs from the price, and keeps the received price as their End price;
  2. saves the Outcomes and Scores in one transaction through the store port;
  3. only after the save succeeds, publishes the resolution events, then the price.

  A guess taken from the index waits in memory with its End price until the save succeeds, so a later trade cannot give it a different End price.
  If the save fails, `game` retries it with the same End price.
  The save is a conditional update that only resolves a guess that is still open, so a retry cannot resolve a guess twice.
- **Order on placing a guess.** `game` saves the guess, adds it to the index, publishes Guess placed to the Player's tabs, then answers the HTTP request.
- **A tab that stops reading.** The price uses one latest-value slot per tab (ADR 0007).
  A tab's Player events go into a queue bounded at 16 events.
  If the queue is full, the adapter closes that tab's stream.
  The client reconnects and gets a new snapshot, which already contains the Score and any Outcome it missed, so no resolution is lost.
- **Keepalive.** The server sends an SSE comment every 15 s.
  A client that receives nothing for 45 s treats the stream as dead and reconnects after a random delay.
  Both values are configuration, with these defaults, and run on `tokio`'s real time like the throttle.

## The `Fanout` port

`Fanout` is a trait in `game` with three operations:

- `publish_price` sends a price event, or a change between live and Price delayed, to every open stream.
- `notify_player` sends an event to every open stream of one Player.
- A subscribe side, which `web` calls when a tab opens its stream, returns the events for that tab's Player and ends when the adapter closes it.

It has two implementations, as ADR 0008 requires of every port:

- **The in-process broadcast**, a module in `game` built on `tokio` channels: a `watch` channel for the price and a bounded channel per tab for Player events, held in a registry keyed by Player ID.
- **A recording fake**, which delivers to subscribers like the real one and also records every published event in order.
  `game`'s tests use it to check, without SSE, for example that a resolution is published only after its save.

Both pass one shared test suite, as the store ports do (ADR 0006).
The port adds no crate, because `game` may already depend on `tokio`.
The name `Fanout` is not a glossary term, because delivering events is infrastructure, not a rule of the game.

The port carries both kinds of event although they scale differently.
Prices are one stream for everyone and suit a broadcast channel or a cache; Player events are small and must be routed to one Player.
For the demo, two traits would be two seams with the same adapter behind them.
The scale path below splits `Fanout` into a price broadcast and Player notifications only when stream servers become separate from the ingesting process.

## Considered Options

- **No cap on price events.** Rejected.
  Bitstamp trades about every 7 s, but in a busy market every trade would go to every tab, and ADR 0003's traffic estimate assumes at most one price a second.
- **A cap per tab instead of one throttle.** Rejected.
  It does the same work once per tab instead of once per server and gives tabs different prices at the same moment.
- **Reading the state over HTTP, then opening the stream, as ADR 0005 first said.** Rejected.
  An event between the read and the stream opening is lost or applied twice, and the snapshot removes that window.
- **Replay through `Last-Event-ID`.** Rejected.
  It needs a per-Player event log on the server, while the snapshot already carries everything a tab shows.
- **Publishing before saving.** Rejected.
  A Player could see an Outcome that a failed save then loses.
- **Choosing a new End price after a failed save.** Rejected.
  ADR 0002 makes the End price the first changed price after the due time, and a retry must not change it.
- **Dropping Player events for a slow tab.** Rejected.
  ADR 0007 forbids dropping resolution events, and closing the stream recovers through the snapshot instead.
- **The fan-out as a plain in-process broadcast with no port, as ADR 0008 first said.** Rejected.
  A port with a recording fake makes the publishing order testable now, and it marks the seam the scale path needs.
- **Two ports, one for prices and one for Player events.** Rejected for the demo; it is a step on the scale path.

## Consequences

- ADR 0003 and ADR 0005 now say that a reconnecting client gets its state from the snapshot.
- ADR 0008 lists `Fanout` as its fifth port.
- If the server restarts while a save is being retried, the guess is still open in the store and resolves against the first changed price after the restart, as ADR 0002 already defines for guesses that fall due during a restart.
- ADR 0014 fixes the event names and payloads.

## Scale path

The numbers come from ADR 0003's scale path, which assumes about one guess a minute per concurrent Player and one price event a second.
The domain core stays the same at every step; what changes is the adapter behind `Fanout`, the store, and how resolution is partitioned.

- **1,000 Players.** One process is still enough, and prices stay on the game server's own stream.
  A managed push service costs far more per message than the bandwidth, and CloudFront cannot share one SSE stream across viewers (ADR 0003).
  `Fanout` keeps its in-process adapter.
- **10,000 Players.** One larger process still carries the connections.
  A deploy drops every stream at once, so clients reconnect with exponential backoff and a random delay of several seconds.
  Every reconnect costs one snapshot: the latest price from memory and one Player read from the store.
  Availability brings the first change, as ADR 0003 describes: with more than one process, `Fanout` gets an adapter over Postgres `LISTEN/NOTIFY` that carries prices and Player events from the leader to every instance.
- **100,000 Players.** One ingesting process, with a standby that takes over through a lease, receives the trades.
  It stamps each price with its receipt time and a sequence number that only grows, and publishes it over a pub/sub channel such as Redis or NATS.
  Stateless SSE servers subscribe to prices and to the Player events of the tabs they hold.
  Resolution is partitioned by Player, and every partition resolves against the same sequence of prices, so the End price of a guess does not depend on which partition holds it.
  The receipt time comes from the ingesting process alone, so one clock still decides the 60-second boundary (ADR 0002).
- **Many guesses due on one price.** Bitstamp can go 30 s without a trade (ADR 0007).
  At 100,000 Players about 1,700 guesses fall due each second, so one trade after a quiet 30 s can resolve about 50,000 guesses at once.
  Each partition saves them in bounded batches, one transaction per batch, and publishes each batch after its save, so the burst spreads over a few seconds instead of one huge transaction.
- **1,000,000 Players and beyond.** As in ADR 0003, this needs its own design.
  Delivering one price a second to a million connections calls for a managed push service or a tree of relays, and the burst on one trade grows to about 500,000 resolutions.
