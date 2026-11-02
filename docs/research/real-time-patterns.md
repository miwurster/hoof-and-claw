# Real-time patterns for price fan-out and guess resolution

Research for [#4](https://github.com/miwurster/hoof-and-claw/issues/4), part of the map [#1](https://github.com/miwurster/hoof-and-claw/issues/1).
Domain terms follow [GLOSSARY.md](../../GLOSSARY.md).

## Question

Which patterns deliver the latest price to browsers and resolve Open guesses on time, at three scales?

- Demo: tens of concurrent players.
- Medium: thousands.
- Large: a million.

Price delivery compares polling, server-sent events (SSE) and WebSockets.
Resolution compares a timer per Open guess, a periodic sweep, and resolving on every price tick.

## Short answer

- **Price delivery.** Use SSE at every scale.
  The flow is one-way (server to browser), SSE reconnects and resumes on its own, and placing a Guess is a plain HTTP POST.
  Polling costs one request per client per interval, which would use up a "one million requests a month" free tier in about 11.6 days of a single open tab polling once per second.
  WebSockets are a valid alternative but add reconnection and replay work that SSE already has.
- **Resolution.** Resolve on the price tick, and use an index on the due time so each tick touches only the Guesses that are due.
  The glossary's End price is "the first price observed at least 60 seconds after the guess was placed that differs from the Start price", so a Guess can only resolve when a price is observed.
  A timer per Guess or a periodic sweep cannot resolve anything without a fresh price, so at best they only trigger fetching one.
- **Bottleneck by scale.** Demo scale has none.
  At medium scale it is running more than one server process: who ingests prices and who resolves.
  At large scale it is the burst of resolution writes and per-player notifications on every tick, plus the connection fan-out itself.

## Facts from primary sources

### Price delivery protocols

- SSE clients reconnect automatically when the connection closes, and the server can stop them with HTTP 204 ([WHATWG HTML, Server-sent events](https://html.spec.whatwg.org/multipage/server-sent-events.html)).
- The server can set the reconnection delay with the `retry` field; the initial value is implementation-defined, "probably in the region of a few seconds" (same source).
- On reconnect, the browser sends the last received event `id` in the `Last-Event-ID` header, so the server can resume (same source).
- The spec warns that legacy proxies drop idle HTTP connections and suggests sending a comment line "every 15 seconds or so" (same source).
- Over HTTP/1.1, browsers limit SSE to 6 open connections per browser and domain across all tabs; over HTTP/2 the limit is the negotiated stream count, 100 by default ([MDN, EventSource](https://developer.mozilla.org/en-US/docs/Web/API/EventSource)).
- The WebSocket opening handshake is an HTTP Upgrade request ([RFC 6455, section 1.3](https://datatracker.ietf.org/doc/html/rfc6455)).
- A WebSocket Ping frame "may serve either as a keepalive or as a means to verify that the remote endpoint is still responsive" ([RFC 6455, section 5.5.2](https://datatracker.ietf.org/doc/html/rfc6455#section-5.5.2)).
- RFC 6455 defines no reconnection or message replay; it only says clients SHOULD back off with a random initial delay after an abnormal closure ([RFC 6455, section 7.2.3](https://datatracker.ietf.org/doc/html/rfc6455#section-7.2.3)).

### Fan-out at scale

- Phoenix reached about 2 million WebSocket connections on one 40-core, 128 GB machine; at about 1.3 million subscribers a broadcast took over 5 seconds until they sharded the subscriber lists, which brought it down to about 1 second ([Phoenix blog](https://www.phoenixframework.org/blog/the-road-to-2-million-websocket-connections)).
- Discord found that fanning one event out from a single process to up to 30,000 sessions took 0.9 to 2.1 seconds at peak, and fixed it by grouping sends per remote node and spreading them over worker processes on that node ("Manifold") ([Discord blog](https://discord.com/blog/how-discord-scaled-elixir-to-5-000-000-concurrent-users)).

### Timers and due-time indexes

- Priority-queue timers (`java.util.Timer`, `DelayQueue`) have O(log n) insert and delete; hierarchical timing wheels have O(1) insert and delete; Kafka switched to timing wheels for tens of thousands of pending operations ([Confluent blog](https://www.confluent.io/blog/apache-kafka-purgatory-hierarchical-timing-wheels/), after Varghese and Lauck, IEEE/ACM ToN 1997).
- A Redis sorted-set range query by score costs O(log(N)+M) for M returned elements ([Redis, ZRANGEBYSCORE](https://redis.io/docs/latest/commands/zrangebyscore/)); the command is deprecated since 6.2 in favour of `ZRANGE ... BYSCORE`.
- Postgres `SELECT ... FOR UPDATE SKIP LOCKED` skips rows other transactions hold locks on; the docs call it unsuitable for general work but useful "to avoid lock contention with multiple consumers accessing a queue-like table" ([PostgreSQL, SELECT](https://www.postgresql.org/docs/current/sql-select.html)).
- Amazon SQS delay queues hold a message back for up to 15 minutes; per-message delays use message timers ([AWS, SQS delay queues](https://docs.aws.amazon.com/AWSSimpleQueueService/latest/SQSDeveloperGuide/sqs-delay-queues.html)).

### Platform features

- A Cloudflare Durable Object has one alarm at a time; alarms run at least once and retry failures with exponential backoff, up to 6 retries starting at 2 seconds ([Cloudflare, Alarms](https://developers.cloudflare.com/durable-objects/api/alarms/)).
- A single Durable Object has a soft limit of 1,000 requests per second; the Free plan offers only SQLite-backed Durable Objects ([Cloudflare, Durable Objects limits](https://developers.cloudflare.com/durable-objects/platform/limits/)).
- With the WebSocket Hibernation API, clients stay connected while the Durable Object is out of memory and no duration charges accrue ([Cloudflare, WebSockets best practices](https://developers.cloudflare.com/durable-objects/best-practices/websockets/)).
- Postgres `NOTIFY` payloads must be under 8000 bytes by default; notifications are delivered only on commit, only to sessions that are listening, and in commit order; a full queue (8 GB by default) makes `NOTIFY` fail at commit ([PostgreSQL, NOTIFY](https://www.postgresql.org/docs/current/sql-notify.html)).
- Redis Pub/Sub delivers at most once: a message a subscriber misses during a disconnect "is forever lost"; Redis Streams persist messages and support at-least-once delivery ([Redis, Pub/Sub](https://redis.io/docs/latest/develop/pubsub/)).

### Price feed and clocks

- Coinbase Exchange's `ticker` channel sends an update on every match, `ticker_batch` sends the latest price every 5 seconds if it changed, and the `heartbeat` channel sends a message every second with a sequence number and last trade ID "to verify that no messages were missed" ([Coinbase Exchange, WebSocket channels](https://docs.cdp.coinbase.com/exchange/websocket-feed/channels)).
  This is one example source; the choice of price source belongs to a separate ticket.
- `Date.now()` follows the system clock and can jump with clock adjustments and skew; `performance.now()` is monotonic and never decreases ([MDN, performance.now](https://developer.mozilla.org/en-US/docs/Web/API/Performance/now)).

## Analysis (inferences)

Everything in this section is reasoning from the facts above and from the glossary, not a sourced claim.

### The domain decides most of the resolution question

The End price must be observed at least 60 seconds after placement and must differ from the Start price.
So a Guess resolves on a price observation and never on time alone.
Two consequences follow:

1. **Resolve on tick is the natural pattern.**
   On each price observation `(price, observedAt)`, resolve every Open guess with `dueAt <= observedAt` and `startPrice != price`.
   Guesses whose Start price equals the new price stay open, as the glossary requires.
2. **Timers and sweeps only matter for pull-based feeds.**
   If the server polls the price source, a Guess due at 12:00:00 cannot resolve until the next poll after that.
   A timer at `dueAt` that triggers a fresh fetch shortens that wait.
   With a push feed that ticks every second or faster, the timer adds nothing.

All Guesses resolved by one tick share the same End price, so a tick resolves a batch in one set-based write.
Because of this, many Guesses due in the same second cost one write at demo and medium scale.

### Price delivery compared

| Concern | Polling | SSE | WebSocket |
| --- | --- | --- | --- |
| Direction | Client pulls | Server pushes | Both ways |
| Requests | One per client per interval | One per connection | One per connection |
| Latency | Up to one interval | Push | Push |
| Reconnect and resume | Not needed | Built in (`retry`, `Last-Event-ID`) | Application code |
| Server state | Stateless; can be cached | Open connection per client | Open connection per client |
| Free-tier fit | Poor when billed per request | Good on an always-on server | Good on an always-on server |
| Proxies | No issue | Needs a comment every ~15 s | Needs Ping frames |

The price is one value shared by all players, and newer values replace older ones.
So a lost price update needs no replay: the next tick corrects it.
A player's own events (their Guess resolved, their Score changed) do matter.
The client should treat the stream as a hint and have the client re-read its state from an HTTP endpoint after every reconnect.
Then no transport needs exactly-once delivery.

Throttling the browser stream to at most one price per second (or per change) keeps bandwidth flat no matter how often the exchange trades.
The server still resolves against every price it observes.

### Resolution patterns compared

| Pattern | How it works | Survives restart | Load shape | Fits End price rule |
| --- | --- | --- | --- | --- |
| Timer per Guess | In-memory timer, platform alarm, or delayed message at `dueAt` | Only if the timer lives outside the process (alarm, delay queue) | Even, one event per Guess | Partly: the timer still needs a fresh price, and must re-arm if the price is unchanged |
| Periodic sweep | Every k seconds, query Guesses with `dueAt <= now` | Yes, if Guesses are persisted | Spiky; up to k seconds late | Partly: needs a price observed after `dueAt` |
| Resolve on tick | Each observed price resolves all due Guesses with a different Start price | Yes, if Guesses are persisted | One batch per tick | Yes, directly |

Durable Object alarms and SQS deliver at least once, so they can fire twice.
Resolution must be idempotent: a conditional update such as `... WHERE id = ? AND resolvedAt IS NULL` with the Score change in the same transaction.

### Per scale

#### Demo: tens of players

- **Delivery.** SSE from one always-on process; placing a Guess is a POST.
- **Resolution.** Resolve on tick, in the same process that ingests the price.
- **Data store access.** One indexed range read and one batch update per tick, for example a partial index on `dueAt` for Guesses not yet resolved; the Score update in the same transaction.
  Keeping Open guesses in memory as well is optional at this size.
- **Bottleneck.** None; tens of SSE connections and tens of Open guesses are trivial.
- **Under load or stall.** If the feed stalls, Guesses stay open, which is correct by definition, and the UI shows "Price delayed" (already in the [visual direction](../design/visual-direction.md)).
  The server needs a staleness check (no price for N seconds) so that a stall is visible and alertable.

#### Medium: thousands of players

- **Delivery.** SSE, possibly across several server instances behind a load balancer.
  One instance can hold thousands of connections (the Phoenix numbers are two orders of magnitude above this).
- **Resolution.** Still resolve on tick, but only one instance may ingest the price and resolve, or the work must be split by player.
  Options are a leader lease (for example a Postgres advisory lock) or partitioning by player ID.
- **Price fan-out between instances.** The leader publishes each price to all instances.
  Postgres `LISTEN/NOTIFY` or Redis Pub/Sub are enough because the price is latest-value; at-most-once delivery costs at most one stale tick.
- **Data store access.** The per-tick batch update grows to tens of rows per second on average; `FOR UPDATE SKIP LOCKED` lets several resolvers share a due-guess table without blocking each other.
- **Bottleneck.** Coordination between instances, not throughput.
- **Under load or stall.** A resolver that crashes mid-batch leaves Guesses open; the next tick picks them up because the query is by `dueAt`, not by "what I scheduled".

#### Large: a million players

- **Delivery.** SSE (or WebSockets) on a fleet of connection servers, each with a few tens of thousands of connections or more.
  Price fan-out becomes a tree: one ingester, a pub/sub layer, then per-node broadcast sharded across workers, as in the Discord and Phoenix write-ups.
  Polling at one request per second would be about a million requests per second, so it is ruled out unless an edge cache absorbs it.
- **Resolution.** Shard Open guesses by player ID.
  Each shard keeps a due-time index (Redis sorted set, a timing wheel in memory backed by a store, or a partitioned table) and resolves on each tick that reaches it.
  With a 60-second Guess and every player always having one Open guess, about 16,700 Guesses come due per second, and as many are placed.
- **Data store access.** Write-heavy; each resolution is a Guess update plus a Score update, about 33,000 writes per second at the assumption above, plus the placements.
  Per-player rows avoid hot rows; the cost is the volume, which calls for batching per tick and per shard.
- **Bottleneck.** The per-tick burst of resolution writes and of per-player outcome notifications, routed to whichever connection server holds that player.
- **Under load or stall.** A stall followed by recovery means every Guess due during the stall resolves on the first tick after it, so the burst is as large as the stall is long.
  Resolvers need a bounded batch size and back-pressure, and the UI must tolerate outcomes arriving a few seconds late.

### Platform features that change the picture

- **Durable Object alarms.** One Durable Object per player with one alarm maps onto "a player has at most one Open guess".
  The alarm survives restarts and fires at least once.
  But the alarm still needs a fresh price; if a million objects each ask one central price object, that object exceeds its soft limit of 1,000 requests per second.
  Pushing the price to players (resolve on tick) avoids that.
- **Redis sorted sets.** A sorted set scored by `dueAt` is a ready-made due-time index; `ZRANGE ... BYSCORE` with `LIMIT` gives bounded batches.
- **Postgres `LISTEN/NOTIFY`.** Good enough to fan a latest-value price out to a few instances; not a durable event log, and a slow listener can fill the queue.
- **Queues with delay.** SQS message timers fit a 60-second delay (the maximum is 15 minutes), but still deliver "it is due" rather than a price, at least once.

### Failure modes

- **Price feed stalls.** No Guess resolves, which the glossary allows ("it stays open past 60 seconds for as long as the price has not changed").
  Detect it with a heartbeat (Coinbase sends one per second) or a staleness timer, show "Price delayed", and alert.
  A feed that is connected but silent looks the same as a market without trades, so the heartbeat, not the absence of ticks, is the signal.
- **Server restarts with Open guesses.** Persist each Guess with its Start price, placement time and due time before acknowledging it.
  On start, there is nothing to rebuild beyond loading the due-time index; the first tick after restart resolves all overdue Guesses.
  In-memory timers alone would lose every Open guess.
- **Clock skew between client and server.** The server stamps placement time, due time and every observation with its own clock; the client never sends a time or a price.
  The client draws its countdown from the server's "remaining milliseconds" plus `performance.now()`, not from `Date.now()`, so a wrong client clock cannot misdraw the countdown.
  On the server, the wall clock is needed to persist times across restarts, but it can be stepped by NTP; measuring the 60 seconds against a monotonic clock within one process, and against the wall clock only across restarts, avoids most of that.
- **Many Guesses due in the same second.** Resolve on tick handles them as one set-based update with one End price.
  At large scale, bound the batch per shard and let the remaining Guesses resolve on the next tick, which is still correct because that tick is also "at least 60 seconds after".
  Strictly, a deferred Guess may then get a later End price than one resolved in the same instant; see the open questions.

## Open questions for a decision ticket

1. **Which time counts as "observed"?**
   The exchange's trade timestamp, the time the server received the message, or the time the resolver processed it?
   They differ by network delay and by batching, and the choice decides which price is the End price near the 60-second boundary.
2. **Outages and deferred batches.** If the server was down or deferred a Guess, is the End price the first price the server observes afterwards, or the historical price right after the 60-second mark, fetched from the source's REST history?
   The glossary's "observed" suggests the former, but players may see that as unfair after a long outage.
3. **What does "differs" mean?**
   At which precision are prices compared (for example cents as the source reports them), and is the comparison on the raw string or on a decimal?
4. **Push or pull price source.** Resolve on tick assumes a push feed with frequent ticks.
   If the chosen source is polled (a REST API on a free tier), the poll interval sets the resolution latency and the request budget, and a timer at `dueAt` to trigger a fetch becomes useful.
5. **Single writer at demo scale.** Does the hosting choice allow exactly one always-on process, so ingestion and resolution need no leader election?

## Sources

- WHATWG HTML Living Standard, Server-sent events: https://html.spec.whatwg.org/multipage/server-sent-events.html
- MDN, EventSource: https://developer.mozilla.org/en-US/docs/Web/API/EventSource
- RFC 6455, The WebSocket Protocol: https://datatracker.ietf.org/doc/html/rfc6455
- MDN, Performance.now(): https://developer.mozilla.org/en-US/docs/Web/API/Performance/now
- Phoenix, The Road to 2 Million WebSocket Connections: https://www.phoenixframework.org/blog/the-road-to-2-million-websocket-connections
- Discord, How Discord Scaled Elixir to 5,000,000 Concurrent Users: https://discord.com/blog/how-discord-scaled-elixir-to-5-000-000-concurrent-users
- Confluent, Apache Kafka, Purgatory, and Hierarchical Timing Wheels: https://www.confluent.io/blog/apache-kafka-purgatory-hierarchical-timing-wheels/
- Redis, ZRANGEBYSCORE: https://redis.io/docs/latest/commands/zrangebyscore/
- Redis, Pub/Sub: https://redis.io/docs/latest/develop/pubsub/
- PostgreSQL, NOTIFY: https://www.postgresql.org/docs/current/sql-notify.html
- PostgreSQL, SELECT (locking clause): https://www.postgresql.org/docs/current/sql-select.html
- AWS, Amazon SQS delay queues: https://docs.aws.amazon.com/AWSSimpleQueueService/latest/SQSDeveloperGuide/sqs-delay-queues.html
- Cloudflare, Durable Objects alarms: https://developers.cloudflare.com/durable-objects/api/alarms/
- Cloudflare, Durable Objects limits: https://developers.cloudflare.com/durable-objects/platform/limits/
- Cloudflare, Durable Objects WebSockets: https://developers.cloudflare.com/durable-objects/best-practices/websockets/
- Coinbase Exchange, WebSocket channels: https://docs.cdp.coinbase.com/exchange/websocket-feed/channels
