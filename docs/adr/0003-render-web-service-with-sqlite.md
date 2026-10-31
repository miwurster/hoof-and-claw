# Host the demo as one always-on Render web service with SQLite, with a scale path to AWS

The demo runs as a single Rust process on a Render web service on the Starter plan in Frankfurt, with SQLite on a Render persistent disk, for about 7.25 USD a month.
The process serves the React build and the API from `hoof-and-claw.miwurster.com`, holds the WebSocket feed to the Price source, pushes prices to browsers over server-sent events (SSE), and resolves due guesses on each price it receives, with no browser open.
The brief prefers AWS, but on AWS an always-on process costs more than the 10 EUR a month this demo may spend, and a serverless AWS design would trade the live feed for polling and a scheduler per guess.
One always-on process is the simplest design that meets every rule of the brief, so the demo uses Render, and AWS is the planned next step once one instance is no longer enough.

## Considered Options

- **AWS serverless: CloudFront, S3, Lambda in Rust, DynamoDB, an EventBridge Scheduler call per guess.** Not chosen for the demo; it is the first candidate on the scale path.
  It costs about 0 to 2 USD a month and matches "AWS preferred".
  A scheduler call per guess is too coarse: EventBridge Scheduler fires with 60-second precision, so a guess nobody watches resolves up to about two minutes after it falls due.
  A Rust Lambda can instead hold the price feed in overlapping 15-minute runs started every 13 minutes, within the free tier, and resolve due guesses on every trade ("AWS serverless for the game", branch `research/aws-serverless-for-the-game`).
  It still gives up one clock, because accepting a guess and receiving prices run on different machines (ADR 0002), and push, because browsers poll a cached price; CloudFront cannot share one SSE stream across viewers.
  A crashed run can leave the feed down for minutes.
  Push services such as AppSync Events or IoT Core charge about 1 USD per million messages, so one price a second to 1,000 Players costs about 2,600 USD a month.
- **AWS always-on: App Runner or ECS Fargate.** Rejected.
  App Runner costs about 14 USD a month, and Fargate needs a load balancer that pushes it past 20 USD.
- **Cloudflare Workers with Durable Objects.** Rejected.
  A Worker on `hoof-and-claw.miwurster.com` requires moving the whole `miwurster.com` zone from All-Inkl to Cloudflare, because subdomain and partial setups need a Business or Enterprise plan.
  An outbound WebSocket or an SSE stream also keeps a Durable Object awake, and one object awake all day uses about 83 % of the free 13,000 GB-s a day.
- **Google Cloud Run with one minimum instance.** Rejected.
  It costs about 4.50 USD a month, but request-based CPU cannot hold a feed, so it has the same polling design as AWS serverless without being AWS.
- **Vercel.** Rejected.
  Hobby is for non-commercial use only, and Pro costs 20 USD a month.
- **A virtual machine (Oracle Always Free, Google `e2-micro`).** Rejected.
  We would patch and run the operating system ourselves, and Oracle may reclaim an idle instance.
- **Render Postgres instead of SQLite.** Rejected for the demo.
  It adds 6 USD a month, about 13 USD in total, and a network hop that one instance does not need.

## Consequences

- A Render service with a disk runs exactly one instance, so price ingestion and resolution need no leader election.
- Every deploy briefly takes the game down, and so does every platform maintenance window, which Render announces and holds at most about once every three months.
  Open guesses survive, because the server persists a guess before it acknowledges it, and they resolve on the first changed price after the restart (see ADR 0002).
- SSE streams and the feed close on every deploy.
  Clients reconnect and rebuild their state from the snapshot that opens every stream (ADR 0010), and the server sends SSE heartbeats, because Render documents no timeout for streaming responses.
  A check run by hand after the first Release holds an SSE stream open for two hours against the deployed service (ADR 0013).
- The server pushes at most one price per second to browsers, and only when it changed.
  Resolution still uses every price it receives.
  At 20 viewers for 4 hours a day this is about 1.3 GB a month, inside the 5 GB Render includes.
  Past that, Render bills 0.15 USD per GB.
- Persistence sits behind a store port whose operations follow the game's access patterns, not SQL.
  They load a Player, place a guess if none is open, and resolve a guess idempotently while adjusting the Score.
  The SQLite adapter runs in WAL mode with a single writing connection.
  Open guesses also live in an in-memory index by due time, rebuilt from the store at start-up, so resolving on a price needs no query.
- There is no database backup.
  Render snapshots the disk daily, but Render advises against restoring a database from a snapshot, so losing the disk resets every Score to 0.
  For a three-month demo we accept that risk.
  The scale path adds continuous replication to object storage, for example with Litestream.
- Render builds the Dockerfile from a `release` branch and deploys it; a Release fast-forwards `release` to a `main` commit whose Green gate passed, and every Release spends Render build minutes (ADR 0013).
- Terraform with the `render-oss/render` provider, applied by hand, describes the service, its `release` branch, disk, domain, health check and environment, and keeps its state in HCP Terraform Free.
  All-Inkl has no Terraform provider, so the `CNAME` from `hoof-and-claw` to the service's `onrender.com` host is a one-time manual step.
- Render shares outbound IP addresses per region.
  A probe on 2026-10-31 confirmed that Bitstamp's feed and REST ticker accept connections from Render Frankfurt ("Check the price feeds from Render Frankfurt").
  If Bitstamp starts refusing them, the fallback is another Render region, then another source (ADR 0004).
- `miwurster.com` expires on 2027-03-22 and renews automatically; if renewal failed, the demo's address would go with it.

## Scale path

The numbers assume each concurrent Player places about one guess a minute, which is close to the maximum because a guess stays open for at least 60 seconds.
That gives about one guess a second per 60 Players, and about 2.5 writes per guess for the guess itself, the Player on a first guess, and the resolution with its Score.
The server pushes one price a second, at about 150 bytes per SSE message including overhead, and the traffic column assumes viewers connected around the clock.
The server never stores prices, only the Start and End price of each guess.

| Concurrent Players | Guesses a second | Writes a second | SSE connections | Outbound traffic |
| --- | --- | --- | --- | --- |
| 1,000 | ~17 | ~40 | 1,000 | ~150 KB/s, ~390 GB a month |
| 10,000 | ~170 | ~420 | 10,000 | ~1.5 MB/s, ~4 TB a month |
| 100,000 | ~1,700 | ~4,200 | 100,000 | ~15 MB/s, ~40 TB a month |
| 1,000,000 | ~17,000 | ~42,000 | 1,000,000 | ~150 MB/s, ~390 TB a month |

The load is neither read- nor write-heavy, just small.
Reads are keyed lookups of one Player, and resolution reads from memory.
SQLite in WAL mode handles thousands of writes a second on an SSD, so the store is not the first bottleneck.
Outbound traffic is the first bottleneck.
At Render's 0.15 USD per GB, 1,000 viewers around the clock cost about 58 USD a month and 10,000 about 600 USD.
The Price source does not grow with the number of Players, because the server needs one feed at every scale.

What each step means for the application:

- **1,000 Players.** The same single process and SQLite are enough on a larger Render instance.
  Prices stay on the game server's own SSE stream.
  A managed push service such as AppSync Events or IoT Core charges about 1 USD per million messages, about 2,600 USD a month for one price a second to 1,000 Players, and CloudFront cannot share one SSE stream across viewers, so bandwidth on the game server stays the cheapest channel ("Scaling the game process" and "AWS serverless for the game", branches `research/scaling-the-game-process` and `research/aws-serverless-for-the-game`).
- **10,000 Players.** One larger process with 2 CPU and 2 to 4 GB still carries the connections and about 420 writes a second, with writes batched into one transaction per price.
  It has no redundancy, and a deploy drops 10,000 connections at once, so clients reconnect with random delay.
  Availability forces a change before capacity does: Render runs zero-downtime deploys only without a disk, so SQLite gives way to managed Postgres behind the store port, and a Postgres advisory lock picks the one process that holds the feed and resolves.
  A second instance follows at about 5,000 sustained Players or when one failing instance is no longer acceptable; Postgres `LISTEN/NOTIFY` carries prices and Player events between instances, and clients need no sticky sessions because every stream opens with a snapshot.
  This can stay on Render or move to AWS or another cloud; the reasons to move are traffic cost per GB, failover across availability zones, and where the organisation already runs.
- **100,000 Players.** One process is no longer enough.
  One ingesting process, with a standby that takes over through a lease, publishes each price to several stateless SSE servers over a pub/sub channel, for example Redis or NATS.
  Guesses move to Postgres or DynamoDB, and resolution is partitioned by Player, each partition with its own index of due guesses.
  Prices carry a sequence number from the ingesting process, so every partition resolves against the same prices.
- **1,000,000 Players and beyond.** This needs its own design work, beyond what this ADR can settle.
  The open questions are how to deliver one price to a million connections affordably, how to shard storage and resolution, and how to absorb the burst when many guesses fall due on the same price.

The domain core stays the same at every step.
The adapters change behind the ports: the store, the delivery of prices to clients, and how resolution is partitioned.
