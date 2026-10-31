# Scaling the game process: bottlenecks and the step to several instances

Research for the scale path in [ADR 0003](../adr/0003-render-web-service-with-sqlite.md) and [ADR 0010](../adr/0010-sse-snapshot-stream-and-fanout-port.md).
Domain terms follow [GLOSSARY.md](../../GLOSSARY.md).
Cost is out of scope, except for one rough price for a Cloud Run instance.

## Question

1. What are the technical bottlenecks of the one Rust process (axum, tokio, SQLite in WAL with one writer) at 1,000, 10,000 and 100,000 concurrent Players?
2. Which limits do Render, AWS (ALB, NLB, ECS Fargate) and Google Cloud Run set for long-lived SSE streams, scaling and deploys?
3. From what point do several instances make sense, separately for availability and for capacity?
4. What does a second instance mean for coordination between instances and with clients?
5. What is the smallest change from one instance to two on Render and on AWS, and what triggers it?

## Short answer

- **One process is not the capacity bottleneck below about 10,000 Players.**
  A prototype of the ADR 0010 fan-out held 10,000 SSE streams in about 320 MB of resident memory and used about 0.2 of a CPU core to push one price a second to all of them (local measurement, see below).
  SQLite in WAL mode with one writer wrote 2.5 to 4.5 million rows a second in batches of 100 to 10,000 on a laptop SSD, far above the 420 writes a second ADR 0003 expects at 10,000 Players.
- **Availability forces the second instance long before capacity does.**
  On Render, a service with a disk runs one instance, has no zero-downtime deploys, and is offline for a few seconds per deploy and a few minutes per maintenance window.
  Every deploy closes every stream at once.
- **A second instance needs the database off the local disk first.**
  SQLite's WAL works only on one host, and Render does not scale a service with a disk.
  So the first step is Postgres behind the store port, not more CPU.
- **With two instances, one of them must lead.**
  One leader, chosen through a lease, ingests the Bitstamp feed, stamps prices and resolves; both instances hold SSE streams.
  Prices and Player events cross instances over a pub/sub channel; Postgres `LISTEN/NOTIFY` is enough at two instances.
  The snapshot that opens every stream means clients need no sticky sessions.
- **Capacity becomes the reason for more instances at roughly 30,000 to 50,000 concurrent streams per process (estimate), or at 1,000 streams per instance on Cloud Run, whose hard concurrency limit is 1,000.**
- **Trigger for two instances.** The first time a deploy or maintenance window during play is not acceptable, or the first sustained 5,000 concurrent Players, whichever comes first.

## Facts from primary sources

### Rust, hyper and the kernel

- A tokio task needs "only a single allocation and 64 bytes of memory" ([Tokio tutorial, Spawning](https://tokio.rs/tokio/tutorial/spawning)).
- hyper's HTTP/1 connection starts with an 8,192-byte read buffer (`INIT_BUFFER_SIZE`) and may grow it to 417,792 bytes (`DEFAULT_MAX_BUFFER_SIZE`) ([hyper `src/proto/h1/io.rs`](https://github.com/hyperium/hyper/blob/master/src/proto/h1/io.rs); [hyper `http1::Builder::max_buf_size`](https://docs.rs/hyper/latest/hyper/server/conn/http1/struct.Builder.html)).
- On Linux, each TCP socket's send buffer starts at 16 KB (`tcp_wmem` default) and its receive buffer at 131,072 bytes (`tcp_rmem` default); both are auto-tuned limits, not up-front allocations ([Linux kernel, ip-sysctl](https://docs.kernel.org/networking/ip-sysctl.html)).

### SQLite

- "SQLite will easily do 50,000 or more INSERT statements per second … But it will only do a few dozen transactions per second" on a spinning disk; the 2024 update says modern SSDs do far more, but grouping operations in one transaction still "can improve performance dramatically" ([SQLite FAQ, question 19](https://www.sqlite.org/faq.html#q19)).
- Only one process can change the database at a time; a writer locks the whole file, "normally only … a few milliseconds" ([SQLite FAQ, question 5](https://www.sqlite.org/faq.html#q5)).
- In WAL mode "there can only be one writer at a time", readers and the writer do not block each other, and "WAL does not work over a network filesystem"; all processes must be on the same host ([SQLite, Write-Ahead Logging](https://www.sqlite.org/wal.html)).
- With `synchronous=NORMAL` in WAL mode only checkpoints call `fsync()`, and committed transactions "might rollback following a power failure" (same source).

### Render

- Compute plans for web services range from Starter (0.5 CPU, 512 MB) through Standard (1 CPU, 2 GB) and Pro (2 CPU, 4 GB) to 12 CPU and 96 GB ([Render, Compute plans](https://render.com/docs/compute-plans)).
- Manual scaling works on every paid workspace, up to 100 instances for plans up to 8 CPU and 32 GB; autoscaling on CPU or memory needs a Pro workspace or higher ([Render, Scaling](https://render.com/docs/scaling)).
- "Services with a persistent disk cannot scale to multiple instances" (same source); "You can't scale a service to multiple instances if it has a disk attached" and "Adding a disk to a service prevents zero-downtime deploys" ([Render, Persistent disks](https://render.com/docs/disks)).
- With a disk, "Render stops the existing instance before bringing up the new instance", which "takes a few seconds, during which your service is unavailable" (same source).
- Without a disk, the old instance keeps all traffic while the new one starts; 60 seconds after traffic moves, Render sends `SIGTERM`, then `SIGKILL` after the shutdown delay, 30 s by default and at most 300 s through `maxShutdownDelaySeconds`; a service with a disk cannot change the delay ([Render, Deploys](https://render.com/docs/deploys)).
- Platform maintenance takes a service with a disk offline for "usually … a few minutes"; services without a disk "remain available"; such maintenance happens "no more than once every three months" ([Render, Platform maintenance](https://render.com/docs/platform-maintenance)).
- Render "load-balances that traffic evenly across all running instances" ([Render, Scaling](https://render.com/docs/scaling)); for WebSockets it "assigns each incoming WebSocket connection to a random instance", documents no sticky sessions, and says "Clients are not guaranteed to reconnect to the same instance" ([Render, WebSockets](https://render.com/docs/websocket)).
- Render enforces no maximum WebSocket duration; connections close when the instance is replaced, and Render recommends reconnecting with exponential backoff (same source).
- "Render allows responses to take up to 100 minutes for HTTP requests" ([Render vs Heroku](https://render.com/docs/render-vs-heroku-comparison)); no page says whether this cap applies to a streaming response such as SSE.
- Render's load balancer terminates TLS and forwards plain HTTP to the service's port ([Render, Web services](https://render.com/docs/web-services)).
- Render Key Value runs Valkey 8 and is "compatible with virtually all clients that interact with Redis"; it is reachable over the private network inside a workspace and region ([Render, Key Value](https://render.com/docs/key-value)).
  Render's page does not mention pub/sub; Valkey itself has `PUBLISH`/`SUBSCRIBE`.
- Render Postgres can "automatically swap to a standby database when your primary encounters an issue", and offers read replicas ([Render, Postgres](https://render.com/docs/postgresql)).
- Render's edge cache for paid web services caches `GET` responses by `Cache-Control`, `s-maxage` taking precedence; with "Common static files" other types "are never cached" ([Render, Edge caching for web services](https://render.com/docs/web-service-caching)).
  Render documents no push or broadcast service.

### AWS: ALB, NLB, ECS Fargate

- ALB idle timeout: 60 s by default, 1 to 4,000 s; a connection is idle when "no data [is] being sent or received"; HTTP/2 PING frames do not reset it ([ALB, Edit load balancer attributes](https://docs.aws.amazon.com/elasticloadbalancing/latest/application/edit-load-balancer-attributes.html)).
- ALB HTTP client keepalive duration: 3,600 s by default, 60 s to 7 days; after it elapses the ALB "accepts one more request and then returns a response that gracefully closes the connection" (same source).
- ALB deregistration delay: 300 s by default; a deregistering target is `draining` and gets no new requests; "If a deregistering target terminates the connection before the deregistration delay elapses, the client receives a 500-level error response" ([ALB, Target group attributes](https://docs.aws.amazon.com/elasticloadbalancing/latest/application/edit-target-group-attributes.html)).
- ALB routing is round robin by default, with least outstanding requests and weighted random as options; sticky sessions use an `AWSALB` cookie (1 s to 7 days) or an application cookie (same source).
- ALB quotas: 1,000 targets per load balancer and per target group by default; no quota on concurrent connections ([ALB quotas](https://docs.aws.amazon.com/elasticloadbalancing/latest/application/load-balancer-limits.html)).
  One ALB capacity unit covers "3,000 active connections per minute" and "25 new connections per second" ([ELB pricing](https://aws.amazon.com/elasticloadbalancing/pricing/)).
- NLB TCP idle timeout: 350 s by default, 60 to 6,000 s; fixed at 350 s for TLS listeners ([NLB, TCP idle timeout](https://docs.aws.amazon.com/elasticloadbalancing/latest/network/update-idle-timeout.html)).
- NLB draining keeps a non-idle connection to a healthy deregistered target open unless "connection termination" is enabled; with client IP preservation off, an NLB supports about 55,000 simultaneous connections per target IP and port ([NLB, Target group attributes](https://docs.aws.amazon.com/elasticloadbalancing/latest/network/edit-target-group-attributes.html)).
- ECS on Fargate: `stopTimeout` defaults to 30 s, at most 120 s; `nofile` defaults to 65,535 and can be raised to 1,048,576 ([ECS task definition parameters](https://docs.aws.amazon.com/AmazonECS/latest/developerguide/task_definition_parameters.html)).

### Google Cloud Run

- Request timeout: 5 minutes by default, at most 60 minutes; when it passes, Cloud Run "closes the network connection", and a reconnecting client "isn't guaranteed to connect to the same instance" ([Cloud Run, Request timeout](https://docs.cloud.google.com/run/docs/configuring/request-timeout)).
- A long-lived stream is a request: Cloud Run caps them at "up to 1,000 per container" and they are "still subject to the request timeout"; session affinity is "best effort", and Cloud Run recommends Redis pub/sub on Memorystore or Firestore real-time updates to share data between instances ([Cloud Run, WebSockets](https://docs.cloud.google.com/run/docs/triggering/websockets)).
- Maximum concurrency per instance is 1,000 ([Cloud Run, Concurrency](https://docs.cloud.google.com/run/docs/about-concurrency)).
- Instance-based billing allocates CPU "for the entire container instance lifecycle", also outside requests; but "Idle instances, including those kept warm using minimum instances, can be shut down at any time", with 10 s of grace after `SIGTERM` ([Cloud Run, Billing settings](https://docs.cloud.google.com/run/docs/configuring/billing-settings)).
- Below 1 vCPU, "The billing settings must be set to request-based billing", so an instance-billed service has at least 1 vCPU ([Cloud Run, CPU](https://docs.cloud.google.com/run/docs/configuring/services/cpu)).
- The official pricing page did not render for an automated read.
  Third-party guides quote Tier 1 instance-based rates of 0.000018 USD per vCPU-second and 0.000002 USD per GiB-second, with a monthly free tier of 240,000 vCPU-seconds and 450,000 GiB-seconds ([CloudCostKit](https://cloudcostkit.com/guides/gcp-cloud-run-pricing/), [Cloudchipr](https://cloudchipr.com/blog/cloud-run-pricing)); treat these as unverified.

### Coordination primitives

- A session-level Postgres advisory lock "is held until explicitly released or the session ends" ([PostgreSQL, Explicit locking](https://www.postgresql.org/docs/current/explicit-locking.html#ADVISORY-LOCKS)).
- Postgres `NOTIFY` payloads must be under 8,000 bytes by default, and notifications reach only sessions listening at commit ([PostgreSQL, NOTIFY](https://www.postgresql.org/docs/current/sql-notify.html)).
- Redis pub/sub delivers at most once; a message a subscriber misses during a disconnect is lost ([Redis, Pub/Sub](https://redis.io/docs/latest/develop/pubsub/)).

### Fan-out at scale, from earlier research

- Phoenix held about 2 million WebSocket connections on one 40-core machine; a broadcast to 1.3 million subscribers took over 5 s until they sharded subscriber lists, then about 1 s ([Phoenix blog](https://www.phoenixframework.org/blog/the-road-to-2-million-websocket-connections)).
- Discord's fan-out from one process to up to 30,000 sessions took 0.9 to 2.1 s at peak before they spread the sends over workers ([Discord blog](https://discord.com/blog/how-discord-scaled-elixir-to-5-000-000-concurrent-users)).

## Local measurement

A throwaway prototype (axum 0.8, tokio, `watch` channel holding a pre-serialised `Bytes` price, an idle `mpsc` queue of 16 per tab, keepalive 15 s) ran on a 10-core Apple laptop with `TOKIO_WORKER_THREADS=2`.
A load client on the same machine opened N plain HTTP/1.1 SSE streams over loopback; the server pushed a 160-byte price event once a second.
Numbers are from one run each; treat them as orders of magnitude, not budgets.

| Streams | Server RSS | Server CPU | Fan-out, median p50 | Fan-out, median of per-tick max |
| --- | --- | --- | --- | --- |
| 1,000 | 35 MB | ~4.5 % of one core | 12 ms | 22 ms |
| 5,000 | 162 MB | not sampled | 30 ms | 44 ms |
| 10,000 | 322 MB | ~20 % of one core | 60 ms | 99 ms |

- Resident memory grew by about 32 KB per stream.
  Most of that is hyper's 8 KB read buffer, its write buffer, axum's per-event buffer and the stream's state; the kernel's socket buffers are not in that figure.
  On Linux they are charged to the container's memory cgroup.
- CPU cost was about 20 µs per stream per event at 10,000 streams, mostly the `write` syscall per stream; serialising once and sharing `Bytes` leaves only a 160-byte copy per stream.
- At 10,000 streams, 1 of 19 ticks had a straggler arriving after 7.5 s.
  The client runs on the same cores, so this may be the client; it is a reason to measure the tail on Linux before relying on it.
- SQLite (rusqlite, WAL, one connection) resolving guesses with a conditional `UPDATE … WHERE resolved_at IS NULL` plus a Score update, two rows per guess:

| Guesses per transaction | Rows a second, `synchronous=NORMAL` | Time per transaction |
| --- | --- | --- |
| 1 | 110,000 | 0.02 ms |
| 100 | 2,600,000 | 0.08 ms |
| 1,000 | 4,200,000 | 0.5 ms |
| 10,000 | 4,500,000 | 4.4 ms |

macOS `fsync` does not flush the drive cache, so `synchronous=FULL` looked almost as fast; on a network-attached cloud disk each sync costs more (see estimates).

## Analysis (inferences and estimates)

### Bottlenecks of one process

| Concern | 1,000 Players | 10,000 Players | 100,000 Players |
| --- | --- | --- | --- |
| Memory for streams | ~35 MB, fits Starter's 512 MB | ~320 MB plus kernel buffers; needs Standard (2 GB) | ~3.2 GB plus kernel buffers (estimate); 8 GB plan |
| File descriptors | ~1,000 | ~10,000 | ~100,000; above Fargate's default 65,535, raise `nofile` |
| CPU for the price push | negligible | ~0.2 core | ~2 cores (estimate, linear) |
| Fan-out to the last stream | ~20 ms | ~100 ms | ~1 s with 2 workers, less with more (estimate) |
| Writes | ~40/s | ~420/s | ~4,200/s, still one batched transaction per trade |
| Resolution burst after 30 s of silence | ~500 guesses | ~5,000 guesses | ~50,000 guesses |

- **Memory and descriptors** grow linearly and stay modest up to 10,000; at 100,000 one process still fits a large instance, but one crash or deploy then drops 100,000 streams.
- **CPU for the push** is the price event times the number of streams; the throttle to one event a second keeps it flat however busy Bitstamp is.
- **Fan-out latency** grows linearly with streams per process, because one `watch` change wakes every stream task and each one writes.
  It stays far below the one-second price period at 10,000; at 100,000 it reaches the period itself, which is when the push needs spreading over processes (estimate, matching Discord's and Phoenix's experience at 30,000 and over 1 million).
- **The resolution burst** is the hardest moment.
  50,000 resolutions are about 100,000 row updates, about 25 ms of SQLite time in 1,000-guess batches on a local SSD.
  On network storage each committed batch also costs a sync of perhaps 1 to 10 ms (estimate), so 50 batches add up to half a second.
  Publishing 50,000 Player events is 50,000 `try_send` calls, then 50,000 stream writes, about one more fan-out.
- **The event loop** is shared: a long synchronous section, such as building one huge transaction or serialising a snapshot per stream on the async threads, stalls every stream on that worker.
  SQLite calls belong on a blocking thread (sqlx already does this), and the burst should yield between batches.
- **SQLite** is not the bottleneck at any of these scales on a local disk.
  Its limits are structural: one host, so it rules out a second instance, and no failover.

### When several instances make sense

- **Availability, Render.**
  With a disk, every deploy is a few seconds offline and every maintenance a few minutes; every stream closes at once.
  Without a disk and with two instances, deploys and maintenance keep the service up, and only the streams on the replaced instance reconnect.
  This is worth it from the first real audience that plays continuously, at any size.
- **Availability, AWS.**
  ECS rolling deploys with an ALB keep the service up from two tasks onward, in two Availability Zones.
- **Capacity.**
  Measured and extrapolated, one process carries about 10,000 streams with room to spare, and roughly 30,000 to 50,000 on 4 CPU and 8 GB before fan-out latency and the blast radius of one restart dominate (estimate).
  So capacity alone calls for a second instance only at tens of thousands of Players, and Cloud Run calls for one per 1,000 streams.

### Coordination with two or more instances

- **Database off the disk.**
  SQLite cannot be shared across hosts, and Render cannot scale a service with a disk; the store port gets a Postgres adapter (Render Postgres or RDS), or DynamoDB on AWS.
- **One leader ingests and resolves.**
  A lease decides it: a Postgres session advisory lock is released when the leader's connection dies, a DynamoDB item with a conditional write and an expiry time does the same on AWS.
  The leader holds the Bitstamp WebSocket, stamps each price with its receipt time and a sequence number, and resolves.
  A follower keeps trying to take the lease; on takeover it reconnects, seeds from the REST ticker (ADR 0007) and rebuilds the due index from the store, as a restart does today.
  Between the old leader's death and takeover the price is Price delayed, which ADR 0007 already covers.
- **A Guess placed on instance B.**
  B holds the request; the 60-second rule needs the Start price and the placement time from the leader's clock (ADR 0002).
  Two ways keep that: B forwards the command to the leader through a request channel and answers when the leader confirms, or B saves it with the leader's latest price and sequence number and the leader stamps the placement time when it takes the guess into its index.
  Stamping on B's own clock is simpler, but it compares two clocks at the boundary, which ADR 0002 rejected.
  The leader learns of new guesses through the same pub/sub, and a periodic query of open guesses by due time catches any it missed, since pub/sub such as `NOTIFY` and Redis is at most once.
- **Player events to the tab on another instance.**
  The leader saves resolutions, then publishes them; every instance subscribes and delivers to the tabs it holds from its registry by Player ID.
  At two instances, broadcast every Player event to every instance; past a handful, use a channel per Player or per partition.
  Postgres `NOTIFY` payloads must stay under 8,000 bytes, and Redis/Valkey pub/sub loses messages during a disconnect, so an instance that lost its subscription closes its streams and lets clients rebuild from the snapshot.
- **Prices to every instance.**
  The leader publishes the throttled price once; each instance puts it into its own `watch` channel.
  At-most-once is fine for a latest-value price.
- **No sticky sessions.**
  The snapshot on every connect carries the whole state, and events carry absolute values (ADR 0010), so a tab may reconnect to any instance.
  The snapshot reads the Player from the shared store and the price from the instance's `watch`.
- **Reconnect storms.**
  On `SIGTERM` an instance should stop accepting streams and close its streams spread over the drain window, not all at once, with an SSE `retry` hint; clients already reconnect after a random delay (ADR 0010).
  The windows are 30 to 300 s on Render, up to 120 s on Fargate, and the ALB's deregistration delay should be at least as long; an ALB answers 5xx when a draining target closes early, which an SSE client simply retries.
- **Idle timeouts.**
  The 15-second keepalive is below the ALB's 60-second default and the NLB's 350 s, so neither cuts a stream; ALB PING frames would not count, SSE comments do.
  The ALB client keepalive (default 1 hour) and Render's 100-minute response cap, if it applies to streams, end a stream at most hourly; the client reconnects as on any close.
- **One clock.**
  Only the leader's clock decides the 60 seconds.
  After a failover the new leader's clock takes over; NTP keeps the jump small, and a guess that falls due during the gap resolves on the first changed price after takeover, as after a restart.

### Platform fit for more than one instance

- **Render.**
  Several web service instances without a disk work, with Render Postgres as the store and either Postgres `LISTEN/NOTIFY` or Render Key Value (Valkey) as pub/sub, all on the private network.
  Render balances streams randomly across instances, with no stickiness, which the snapshot design does not need.
  Render has no push service; its edge cache can serve a short-lived price document (`s-maxage=1`) to polling clients, but it is not a stream, so one-to-many price delivery at very large scale means an external push service.
- **Google Cloud Run with instance-based billing and `min-instances=1`** can keep an outbound WebSocket open, because CPU stays allocated outside requests.
  But Cloud Run may shut even a minimum instance down "at any time", which leaves the feed with gaps (Price delayed) and closes all streams; every SSE stream ends after at most 60 minutes; and one instance takes at most 1,000 streams.
  One always-on instance with 1 vCPU and 512 MiB costs roughly 45 USD a month at the unverified rates above (2.59 million seconds a month, less the free tier).
  It fits the demo technically, but past 1,000 Players it needs the multi-instance design anyway.
- **AWS.**
  ECS Fargate behind an ALB has no hard stream limit and generous timeouts.
  An NLB avoids HTTP processing but has a fixed 350 s idle timeout on TLS listeners and a 55,000-connection limit per target without client IP preservation.

### Building blocks by platform

| Building block | Render | AWS | Google Cloud |
| --- | --- | --- | --- |
| Always-on process | Web service, no disk, several instances | ECS Fargate service behind an ALB | Cloud Run with instance-based billing and `min-instances`, or GKE for strict always-on |
| Managed Postgres | Render Postgres, with standby | RDS or Aurora PostgreSQL; or DynamoDB | Cloud SQL for PostgreSQL |
| Pub/sub between instances | Postgres `LISTEN/NOTIFY`, or Render Key Value (Valkey) | Postgres `LISTEN/NOTIFY`, ElastiCache (Valkey/Redis), or SNS/SQS | Postgres `LISTEN/NOTIFY`, Memorystore (Redis/Valkey), or Pub/Sub |
| Leader lease | Postgres advisory lock | Postgres advisory lock, or DynamoDB conditional write | Postgres advisory lock |
| Price delivery to many clients | Own SSE streams; edge cache for a polled price document; no push service | Own SSE streams; CloudFront for a cached price document; a managed WebSocket or pub/sub service | Own SSE streams; Cloud CDN; Firestore real-time updates |

## Recommendation: the smallest step from one instance to two

The trigger is the first time a deploy or a maintenance window during play is not acceptable, or the first sustained 5,000 concurrent Players (half the measured comfortable load, leaving room for a failover), whichever comes first.

**On Render**

1. Add a Postgres adapter behind the store port and move to Render Postgres; drop the disk.
2. Take leadership through a Postgres session advisory lock; only the leader opens the Bitstamp feed and runs resolution.
3. Use `LISTEN/NOTIFY` on the same database for prices, Player events and new guesses, so no new service is needed; add a periodic due-guess query as the safety net.
4. Scale the service to two instances manually and raise `maxShutdownDelaySeconds` so each instance can spread its stream closes over the drain window.
5. Keep the snapshot-first stream as is; add an SSE `retry` field with jitter.

**On AWS**

The same application change, with RDS PostgreSQL, two ECS Fargate tasks in two Availability Zones behind an ALB, idle timeout left at 60 s or higher, deregistration delay at least as long as `stopTimeout` (at most 120 s), and no stickiness.

Later, for capacity, split `Fanout` into a price broadcast and Player notifications, move pub/sub to Valkey/Redis or NATS once `NOTIFY` traffic grows, and partition resolution by Player, as ADR 0003 and ADR 0010 already sketch.

## Open questions

1. Does Render's 100-minute response cap close SSE streams, and does its proxy hold one upstream connection per client stream? A two-hour `smoke:hold` run would settle the first.
2. What does Render's private network offer for reaching one specific instance, if the leader should receive forwarded commands?
3. How large is the fan-out tail on Linux with the load client on a separate machine?
4. What sync latency does Render Postgres or RDS add per resolution batch, and so how long does the 50,000-guess burst take in practice?

## Sources

- Tokio tutorial, Spawning: https://tokio.rs/tokio/tutorial/spawning
- hyper, `src/proto/h1/io.rs`: https://github.com/hyperium/hyper/blob/master/src/proto/h1/io.rs
- hyper, `http1::Builder`: https://docs.rs/hyper/latest/hyper/server/conn/http1/struct.Builder.html
- Linux kernel, IP sysctl: https://docs.kernel.org/networking/ip-sysctl.html
- SQLite FAQ: https://www.sqlite.org/faq.html
- SQLite, Write-Ahead Logging: https://www.sqlite.org/wal.html
- Render, Compute plans: https://render.com/docs/compute-plans
- Render, Scaling: https://render.com/docs/scaling
- Render, Persistent disks: https://render.com/docs/disks
- Render, Deploys: https://render.com/docs/deploys
- Render, Platform maintenance: https://render.com/docs/platform-maintenance
- Render, WebSockets: https://render.com/docs/websocket
- Render, Web services: https://render.com/docs/web-services
- Render vs Heroku: https://render.com/docs/render-vs-heroku-comparison
- Render, Key Value: https://render.com/docs/key-value
- Render, Postgres: https://render.com/docs/postgresql
- Render, Edge caching for web services: https://render.com/docs/web-service-caching
- AWS, ALB load balancer attributes: https://docs.aws.amazon.com/elasticloadbalancing/latest/application/edit-load-balancer-attributes.html
- AWS, ALB target group attributes: https://docs.aws.amazon.com/elasticloadbalancing/latest/application/edit-target-group-attributes.html
- AWS, ALB quotas: https://docs.aws.amazon.com/elasticloadbalancing/latest/application/load-balancer-limits.html
- AWS, Elastic Load Balancing pricing: https://aws.amazon.com/elasticloadbalancing/pricing/
- AWS, NLB TCP idle timeout: https://docs.aws.amazon.com/elasticloadbalancing/latest/network/update-idle-timeout.html
- AWS, NLB target group attributes: https://docs.aws.amazon.com/elasticloadbalancing/latest/network/edit-target-group-attributes.html
- AWS, ECS task definition parameters for Fargate: https://docs.aws.amazon.com/AmazonECS/latest/developerguide/task_definition_parameters.html
- Google Cloud Run, Request timeout: https://docs.cloud.google.com/run/docs/configuring/request-timeout
- Google Cloud Run, WebSockets: https://docs.cloud.google.com/run/docs/triggering/websockets
- Google Cloud Run, Concurrency: https://docs.cloud.google.com/run/docs/about-concurrency
- Google Cloud Run, Billing settings: https://docs.cloud.google.com/run/docs/configuring/billing-settings
- Google Cloud Run, CPU: https://docs.cloud.google.com/run/docs/configuring/services/cpu
- CloudCostKit, Cloud Run pricing (secondary): https://cloudcostkit.com/guides/gcp-cloud-run-pricing/
- Cloudchipr, Cloud Run pricing (secondary): https://cloudchipr.com/blog/cloud-run-pricing
- PostgreSQL, Explicit locking: https://www.postgresql.org/docs/current/explicit-locking.html
- PostgreSQL, NOTIFY: https://www.postgresql.org/docs/current/sql-notify.html
- Redis, Pub/Sub: https://redis.io/docs/latest/develop/pubsub/
- Phoenix, The Road to 2 Million WebSocket Connections: https://www.phoenixframework.org/blog/the-road-to-2-million-websocket-connections
- Discord, How Discord Scaled Elixir to 5,000,000 Concurrent Users: https://discord.com/blog/how-discord-scaled-elixir-to-5-000-000-concurrent-users
