# SQL or NoSQL for Hoof & Claw?

Research into whether the game's store should stay relational (SQLite now, Postgres next) or move to a NoSQL store.
All sources were read on 2026-10-31.
Terms follow `GLOSSARY.md`.

## Question

Why does a SQL store (SQLite now, Postgres next) fit the game, and when, if ever, would DynamoDB, Firestore, Cassandra or ScyllaDB, Redis as the primary store, or MongoDB fit better?

What the store does today (ADR 0003, ADR 0008, ADR 0010):

- **Load a Player**, by ID: Score and Open guess.
- **Place a Guess**, only if the Player has no Open guess.
- **Resolve a Guess** idempotently, only if it is still open, together with the Score change, in one transaction.
- **Save all resolutions of one trade** in one transaction, then publish them.
- **Rebuild the in-memory index of Open guesses by due time** at start-up.

There is no prices table; each Guess stores its Start price and End price with receipt time and trade id.
The load model from ADR 0003 is about one Guess a minute per concurrent Player and about 2.5 writes per Guess:

| Concurrent Players | Guesses a second | Writes a second |
| --- | --- | --- |
| 1,000 | ~17 | ~40 |
| 10,000 | ~170 | ~420 |
| 100,000 | ~1,700 | ~4,200 |
| 1,000,000 | ~17,000 | ~42,000 |

## Answer in short

For this game, a NoSQL store does not pay off at any scale in ADR 0003's table, with two exceptions that change the shape of the system rather than the load.

- **The rules are relational constraints.** "One Open guess per Player" is one partial unique index in SQLite and Postgres, and "resolve only if still open, with the Score" is one conditional `UPDATE` in a transaction.
  DynamoDB expresses both just as well if the Open guess lives inside the Player item, so the core operations are not what separates the stores.
  What separates them is everything around the core: the start-up query by due time, a leaderboard sorted by Score, accounts with unique sign-in identities, ad-hoc questions, and the shared store-port test suite.
  SQL does all of these with an index or a query.
  DynamoDB and Firestore need a global index per pattern, and the two global patterns here (all Open guesses by due time, all Players by Score) concentrate writes on a few keys, which is where their per-partition limits apply.
- **Throughput is not the reason to leave SQL.**
  On this laptop, SQLite placed about 256 Guesses a second with each commit flushed to the drive (`fullfsync`), and resolved 19,000 to 87,000 Guesses a second in batches of 100 to 1,000; a local Postgres 17 ran about 28,000 full Guess lifecycles a second (place, resolve, Score) with 64 clients.
  These are not cloud numbers, but they show that at 100,000 Players (about 1,700 Guesses and 4,200 row writes a second) the limit is how commits are flushed, not the engine.
  SQLite's real limit is that WAL works only on one host, so it ends where ADR 0003 already ends it: when the game needs a second instance or redundancy, around 10,000 Players.
- **Cost rises fastest on the per-request stores.** DynamoDB on demand costs about 680 USD a month at 10,000 Players and about 6,800 USD at 100,000; Firestore about 980 and 9,800 USD.
  A Render Postgres instance with 2 CPU and 16 GB costs 160 USD a month plus storage.
- **When NoSQL would pay off.**
  1. If the game moves to serverless AWS with no always-on process, as "AWS serverless for the game" researched, DynamoDB is the natural store: it costs about 0 USD at demo scale, needs no connection management and no server.
     That is a change of hosting model, not of data model.
  2. At around 1,000,000 concurrent Players (42,000 writes a second), one Postgres primary is near what a single node can sustain, and the choice is between sharded Postgres (Citus, or hash partitions by Player across nodes) and DynamoDB.
     Every core operation is keyed by Player, so both work; DynamoDB with provisioned, reserved capacity is then a fair contender, and on-demand (about 68,000 USD a month) is not.
- **Never as the primary store.** Redis's default persistence loses up to a second of acknowledged writes, and its transactions do not roll back.
  Firestore is the most expensive, and its limit of 500 writes a second on monotonically increasing indexed fields is reached at about 30,000 Players.
  Redis does fit next to SQL as a leaderboard (a sorted set) or as the pub/sub channel ADR 0010 already names.

## Findings

"Verified" lines come from the cited source.
Lines marked _(inference)_ are reasoning on top of the sources.
Lines marked _(measured)_ come from the local runs in "Local measurements" below.

### 1. The store operations in each store

#### SQL: SQLite and Postgres

Verified:

- SQLite supports partial indexes since 3.8.0, and "a partial index definition may include the UNIQUE keyword", which "provides a mechanism for enforcing uniqueness across some subset of the rows in a table"; the documented example enforces one leader per team with `CREATE UNIQUE INDEX team_leader ON person(team_id) WHERE is_team_leader` ([SQLite partial indexes](https://www.sqlite.org/partialindex.html)).
- Postgres documents the same pattern: a unique partial index "enforces uniqueness among the rows that satisfy the index predicate, without constraining those that do not" ([Postgres partial indexes, Example 11.3](https://www.postgresql.org/docs/current/indexes-partial.html)).
- Postgres `INSERT ... ON CONFLICT` takes an `index_predicate`, "used to allow inference of partial unique indexes", and `RETURNING` returns only rows "successfully inserted or updated" ([Postgres INSERT](https://www.postgresql.org/docs/current/sql-insert.html)).
- SQLite supports `RETURNING` since 3.35.0 ([SQLite RETURNING](https://www.sqlite.org/lang_returning.html)).

The operations, as run on SQLite in "Local measurements"; the Postgres run used the same index and `INSERT`, and resolved with one CTE _(measured)_:

```sql
CREATE UNIQUE INDEX one_open_guess ON guess(player_id) WHERE resolved_at IS NULL;
CREATE INDEX open_by_due ON guess(due_at) WHERE resolved_at IS NULL;

-- Place a guess only if none is open: no row back means refused.
INSERT INTO guess(player_id, direction, start_price, placed_at, due_at) VALUES (?, ?, ?, ?, ?)
  ON CONFLICT (player_id) WHERE resolved_at IS NULL DO NOTHING RETURNING id;

-- Resolve idempotently with the Score, in one transaction.
UPDATE guess SET resolved_at = ?, end_price = ?, outcome = ? WHERE id = ? AND resolved_at IS NULL;
UPDATE player SET score = score + ? WHERE id = ?;  -- only if the first statement changed a row

-- Rebuild the index at start-up.
SELECT id, player_id, start_price, due_at FROM guess WHERE resolved_at IS NULL ORDER BY due_at;
```

- Every operation is one statement or one short transaction, and the rule "one Open guess" holds even if application code has a bug, because the database refuses the second row _(measured on SQLite: a second Open guess for the same Player was refused in every run)_.
- Saving all resolutions of a trade is the same pair of statements repeated inside one transaction; in Postgres it can be one statement over `unnest` arrays with a CTE _(inference)_.
- A retry after a failed save changes nothing, because the `WHERE resolved_at IS NULL` condition matches no row the second time _(inference)_.
- The schema already has every column ADR 0002 needs (Start price, End price, receipt time, trade id), and the same SQL runs on SQLite and Postgres with small dialect differences, so the Postgres adapter is mostly a copy of the SQLite one _(inference)_.

What SQL makes hard: nothing in today's access patterns.
The cost is that SQLite allows one writer at a time, and a Postgres server is one more thing to run.

#### DynamoDB

Verified:

- A condition expression makes a write proceed only if it is true; `attribute_not_exists` on the key prevents overwriting an item, and a conditional `UpdateItem` fails when its condition is false ([condition expressions](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/Expressions.ConditionExpressions.html)).
- `TransactWriteItems` groups "up to 100 write actions" on up to 100 distinct items, at most 4 MB in total, in one Region; one transaction cannot target the same item twice; a client token makes it idempotent for 10 minutes ([transactions](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/transaction-apis.html)).
- "DynamoDB performs two underlying reads or writes of every item in the transaction", and a cancelled transaction still consumes that capacity ([transactions](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/transaction-apis.html)).
- "Transactions cannot be performed using indexes", and a transaction's changes reach global secondary indexes (GSIs) "gradually" ([transactions](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/transaction-apis.html)).
- GSIs are updated "asynchronously, using an eventually consistent model", and support only eventually consistent reads ([GSI](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/GSI.html)).
- A GSI is sparse: an item appears only if it has the index key attributes, the documented example being an `isOpen` attribute removed when an order ships ([sparse indexes](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/bp-indexes-general-sparse-indexes.html)).

The single-table design that fits _(inference)_:

| Operation | DynamoDB | Write units |
| --- | --- | --- |
| Load a Player | `GetItem` on `PK = PLAYER#<id>`, strongly consistent; the item holds the Score and the Open guess as a map | 1 read unit |
| Place a Guess | `UpdateItem` on the Player: `SET openGuess = :g` with condition `attribute_not_exists(openGuess)` | 1 |
| Resolve a Guess with its Score | `UpdateItem` on the Player: `SET score = score + :d REMOVE openGuess` with condition `openGuess.id = :g` | 1 |
| Keep the Resolved guess as history | Add a `Put` of `PK = PLAYER#<id>, SK = GUESS#<id>` and make it a `TransactWriteItems` with the update above | 4 (2 items × 2) |
| Save all resolutions of a trade | Parallel conditional updates, one per Player; at most 100 per transaction if they must commit together | 1 to 4 each |
| Rebuild the index | Placing also sets top-level `openDueBucket` and `openDueAt` on the Player item and resolving removes them; `Query` the resulting sparse GSI bucket by bucket, eventually consistent | 1 GSI write on every place and every resolve |

- Keeping the Open guess inside the Player item turns "one Open guess per Player" and "resolve only if still open, with the Score" into single-item conditional writes of 1 write unit each.
- "One transaction per trade" is a SQL batching choice, not a rule; the rule in ADR 0010 is "publish only after save" and "retry with the same End price", and independent conditional writes keep both.
  At 100,000 Players one trade after a quiet 30 s resolves about 50,000 Guesses (ADR 0010), which is 500 transactions of 100.
- What DynamoDB makes hard: the rebuild query.
  It needs a GSI whose partition key every Open guess shares (or a few dozen sharded buckets of it), because Query needs an equality on the partition key; every place and every resolve then writes that GSI, and a GSI that lacks capacity throttles the base table.
- GSIs follow the table asynchronously, so a rebuild right after a crash can miss a Guess placed a moment before; the server then learns of it only when it loads that Player _(inference)_.

#### Firestore

Verified:

- Transactions are all-or-nothing, reads must come before writes, and the transaction function may run more than once on contention ([transactions](https://firebase.google.com/docs/firestore/manage-data/transactions)).
- Standard edition uses pessimistic concurrency in server client libraries: "a transaction's lock on a document blocks other transactions"; mobile and web SDKs emulate optimistic concurrency ([data contention](https://docs.cloud.google.com/firestore/native/docs/transaction-data-contention)).
- A transaction may run at most 270 seconds, a request at most 10 MiB, and a document at most 1 MiB ([quotas](https://firebase.google.com/docs/firestore/quotas)).
- "Cloud Firestore documents can only be updated at most once per second" ([aggregation](https://firebase.google.com/docs/firestore/solutions/aggregation)), while the best-practices page says the exact rate "depends highly on the workload" ([best practices](https://firebase.google.com/docs/firestore/best-practices)).
- "Cloud Firestore limits the write rate to 500 writes per second" for a collection with "a monotonically increasing or decreasing indexed field", such as a timestamp ([sharded timestamps](https://firebase.google.com/docs/firestore/solutions/shard-timestamp)).
- New collections should start at 500 operations a second and grow by the "500/50/5" rule ([best practices](https://firebase.google.com/docs/firestore/best-practices)).

Fit _(inference)_:

- Place and resolve map to a transaction that reads the Player document, checks the Open guess and writes it, which works.
  A Player's document changes two or three times a minute, inside the per-document limit.
- Guesses carry `placed_at` and `due_at`, both monotonically increasing, so a Guesses collection with those fields indexed is capped at 500 writes a second, about 30,000 Players, unless the indexes are exempted or sharded.
  The rebuild query needs exactly such an index.
- Firestore is built for mobile and web clients talking to the database directly; this game has one server that owns all writes, so the client SDKs, offline cache and security rules bring nothing.

#### Cassandra and ScyllaDB, Redis, MongoDB

Verified:

- Cassandra's `IF NOT EXISTS` and `IF` conditions "incur a non-negligible performance cost, because Paxos is used"; batches are isolated only within one partition; counter updates "are not idempotent" ([Cassandra DML](https://cassandra.apache.org/doc/latest/cassandra/developing/cql/dml.html)).
- Redis transactions run commands in isolation but "Redis does not support rollbacks"; a command that fails after `EXEC` does not stop the others ([Redis transactions](https://redis.io/docs/latest/develop/using-commands/transactions/)).
- Redis's default AOF policy fsyncs every second, so "you may lose 1 second of data if there is a disaster"; `appendfsync always` is "very very slow" ([Redis persistence](https://redis.io/docs/latest/operate/oss_and_stack/management/persistence/)).
- Redis sorted sets are documented for leaderboards, with `ZADD` and `ZINCRBY` in O(log N) ([sorted sets](https://redis.io/docs/latest/develop/data-types/sorted-sets/)).
- MongoDB supports unique partial indexes, where "the unique constraint only applies to the documents that meet the filter expression" ([partial indexes](https://www.mongodb.com/docs/manual/core/index-partial/)).
- On a sharded MongoDB collection, only the shard key index, a compound index prefixed by the shard key, or `_id` can be unique ([shard key indexes](https://www.mongodb.com/docs/manual/core/sharding-shard-key-indexes/)).
- A MongoDB transaction must finish in under a minute by default ([transactions in production](https://www.mongodb.com/docs/manual/core/transactions-production-consideration/)).

Fit _(inference)_:

- **Cassandra and ScyllaDB.** Every core operation is a conditional write, so every one would be a Paxos round trip; the Score cannot be a counter, because counters are not idempotent and resolution must be.
  They pay off for very high write rates with few conditions, which is the opposite of this game.
- **Redis as the primary store.** A Lua script can check the Open guess and update the Score atomically, but a crash loses up to a second of acknowledged Guesses and Outcomes, which breaks ADR 0003's "persist before acknowledging" promise.
  As a leaderboard derived from the SQL store, or as ADR 0010's pub/sub channel, it fits well.
- **MongoDB.** It has everything the game needs (partial unique index, multi-document transactions, sharding by Player ID with the unique index prefixed by it), but for this data model it offers nothing Postgres does not, and Rust has no compile-time query checks for it as sqlx gives for SQL (ADR 0006).

### 2. Throughput limits

#### SQLite in WAL mode

Verified:

- "Since there is only one WAL file, there can only be one writer at a time", and "WAL does not work over a network filesystem" because all processes must share memory on one host ([WAL](https://www.sqlite.org/wal.html)).
- "SQLite will easily do 50,000 or more INSERT statements per second on an average desktop computer. But it will only do a few dozen transactions per second" on a rotating disk; the fix is to batch statements into one transaction ([FAQ 19](https://www.sqlite.org/faq.html)).
- With `synchronous=FULL` in WAL mode "an additional sync operation of the WAL file happens after each transaction commit"; with `NORMAL`, WAL "is safe from corruption" but "a transaction committed ... might roll back following a power loss or system crash" ([PRAGMA synchronous](https://www.sqlite.org/pragma.html)).
- sqlx's default is `synchronous = FULL`, and it does not set a journal mode itself ([sqlx SqliteConnectOptions](https://docs.rs/sqlx/latest/sqlx/sqlite/struct.SqliteConnectOptions.html)).
- The SQLite authors' checklist: choose client/server when the data is separated from the application by a network, when there are "many concurrent writers" that "cannot queue up and take turns", or for data in the terabyte range ([when to use](https://www.sqlite.org/whentouse.html)).

Measured on an Apple M5 laptop SSD, Python's `sqlite3` with SQLite 3.50.4, WAL mode, the schema above and 200,000 Players _(measured)_:

| Flush to media (`fullfsync`) | `synchronous` | Place, one transaction each | Resolve, 1 per transaction | Resolve, 100 per transaction | Resolve, 1,000 per transaction |
| --- | --- | --- | --- | --- | --- |
| Yes | FULL | 256 a second | 256 a second | 19,000 a second | 87,000 a second |
| Yes | NORMAL | 15,000 a second | 11,600 a second | 276,000 a second | 704,000 a second |
| No | FULL | 23,000 a second | 21,000 a second | 462,000 a second | 738,000 a second |

- With every commit flushed to media, the rate of single-guess transactions is the rate of flushes: about 256 a second here.
  macOS's plain `fsync` does not flush the drive's cache, which is why the "No" rows are so much faster; Linux's `fsync` on a cloud volume behaves like the "Yes" rows, at a rate set by the volume's flush latency _(inference)_.
- So with today's defaults (sqlx's `FULL`, one transaction per placed Guess), the store's ceiling for placing Guesses is roughly one flush per Guess: comfortable at 1,000 Players (17 a second), plausible at 10,000 (170 a second), not at 100,000 (1,700 a second) _(inference)_.
- Two cheap changes lift that ceiling by two orders of magnitude without leaving SQLite: let the single writer group the Guesses that arrive within a few milliseconds into one transaction, as resolutions already are, or accept `synchronous=NORMAL`, which keeps the file consistent but can lose the last commits on a power loss _(inference from the measurements)_.
- Rebuilding the index of 10,000 Open guesses from the partial index took 2 to 3 ms _(measured)_.

#### Postgres on one instance

Verified:

- Commit waits for the WAL flush by default, and "for short transactions this delay is a major component of the total transaction time"; `commit_delay` lets "a single flush executed by one such transaction ... also serve other transactions committing at about the same time" (group commit), while asynchronous commit drops the wait at the cost of the latest transactions ([async commit](https://www.postgresql.org/docs/current/wal-async-commit.html)).
- RDS gp3 storage gives a baseline of 3,000 IOPS and 125 MiB/s below 400 GiB, and 12,000 IOPS, provisionable to 64,000, from 400 GiB; io2 Block Express goes to 256,000 IOPS ([RDS storage](https://docs.aws.amazon.com/AmazonRDS/latest/UserGuide/CHAP_Storage.html)).
- Render Postgres compute plans run from 0.1 CPU and 256 MB to 128 CPU and 1,024 GB, with 100 to 500 connections ([Render compute plans](https://render.com/docs/compute-plans)).
- Render Postgres read replicas need at least 0.5 CPU and 10 GB, up to five per database; high availability needs at least 1 CPU, fails over after 30 s of unavailability, and can lose "no more than a few seconds of changes" because replication is asynchronous ([read replicas](https://render.com/docs/postgresql-read-replicas), [high availability](https://render.com/docs/postgresql-high-availability)).
- Cloud SQL dedicated-core instances hold up to 64 TB and a table at most 32 TB ([Cloud SQL quotas](https://docs.cloud.google.com/sql/docs/postgres/quotas)).
- Postgres's own rule of thumb for partitioning: worthwhile "only when a table would otherwise be very large", roughly when "the size of the table should exceed the physical memory of the database server" ([partitioning](https://www.postgresql.org/docs/current/ddl-partitioning.html)).
- Citus is not a fit "when you do not expect your workload to ever grow beyond a single Postgres node" ([what is Citus](https://docs.citusdata.com/en/stable/get_started/what_is_citus.html)).

Measured with Postgres 17 in Docker on the same laptop (10 CPUs to the VM), `pgbench` running the place and resolve statements above as two transactions per Guess, against 1,000,000 Players _(measured)_:

| Clients | Guess lifecycles a second | Row writes a second (×3) | Mean latency |
| --- | --- | --- | --- |
| 1 | 4,100 | 12,300 | 0.24 ms |
| 16 | 16,500 | 49,400 | 0.97 ms |
| 64 | 28,500 | 85,400 | 2.25 ms |

- Docker's VM disk does not flush like a cloud volume, so these are upper bounds, not a forecast _(inference)_.
  The useful result is the shape: with many concurrent clients, Postgres group-commits, so throughput keeps rising when each commit costs a flush.
- At 100,000 Players (1,700 Guesses, 4,200 writes a second) one mid-sized managed instance is enough; read replicas add nothing, because every read is a keyed lookup of one Player, and only reconnect storms after a deploy (ADR 0010) push reads up _(inference)_.
- At 1,000,000 Players (17,000 Guesses, 42,000 writes a second) a single primary is plausible on large hardware with batched commits, but it is where the design needs its own load test, as ADR 0003 already says _(inference)_.
- The first thing to outgrow one node is not writes a second but storage, if every Resolved guess is kept: at roughly 100 bytes a row, 100,000 Players add about 4.4 billion Guesses and several hundred GB a month, so the Guess table passes the server's memory within weeks and wants monthly partitions dropped or archived; 1,000,000 Players add about 4 TB a month _(inference)_.
- Every operation is keyed by Player, so if one node runs out, distributing by Player ID (Citus, or the "resolution partitioned by Player" of ADR 0003 with one Postgres per partition) keeps every transaction on one node _(inference)_.

#### DynamoDB

Verified:

- "Every partition in a DynamoDB table is designed to deliver a maximum capacity of 3,000 read units per second and 1,000 write units per second", one write unit being one write of up to 1 KB ([partition key design](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/bp-partition-key-design.html)).
- Write sharding spreads a hot key over suffixes such as `.1` to `.200`, and reading "all the items" then means querying every suffix and merging ([write sharding](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/bp-partition-key-sharding.html)).
- New on-demand tables sustain "up to 4,000 writes per second and 12,000 reads per second", accommodate "up to double the previous peak", and may throttle if traffic exceeds double the peak within 30 minutes ([on-demand mode](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/on-demand-capacity-mode.html)).
- The default table quota is 40,000 read and 40,000 write units a second in either mode, adjustable ([quotas](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/ServiceQuotas.html)).
- A GSI write costs one unit to add an entry and two when an index key changes (delete and put); a table write fails if any of its GSIs lacks capacity ([GSI](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/GSI.html)).
- On demand in `us-east-1`: 0.625 USD per million write request units, 0.125 USD per million read request units, 0.25 USD per GB-month; transactional writes take 2 units per KB ([on-demand pricing](https://aws.amazon.com/dynamodb/pricing/on-demand/)).
- Provisioned in `us-east-1`: 0.00065 USD per write unit-hour and 0.00013 USD per read unit-hour, 25 of each free, with reserved capacity up to 54 % cheaper for one year and 77 % for three ([provisioned pricing](https://aws.amazon.com/dynamodb/pricing/provisioned/)).

Base-table write cost a month (30 days), one unit per write, no GSIs, `us-east-1` prices _(inference)_:

| Concurrent Players | Writes a second | On demand | Provisioned, steady | Provisioned, 1-year reserved (−54 %) |
| --- | --- | --- | --- | --- |
| 1,000 | 40 | 65 USD | 7 USD | n/a |
| 10,000 | 420 | 680 USD | 197 USD | 91 USD |
| 100,000 | 4,200 | 6,800 USD | 1,970 USD | 900 USD |
| 1,000,000 | 42,000 | 68,000 USD | 19,700 USD | 9,000 USD |

- Transactions double these figures for the writes they cover; the Open-guess GSI adds about two units per Guess, and a leaderboard GSI two per Score change.
- Provisioned capacity must also cover the burst after a quiet market (50,000 resolutions on one trade at 100,000 Players, ADR 0010) or the writes throttle and retry, so steady figures are a floor.
- Per-Player items spread over the whole key space, so the 1,000-unit partition limit never binds the base table; it binds the two global patterns.
  An Open-guess GSI keyed by due-time bucket receives about 3,400 writes a second at 100,000 Players (1,700 adds and 1,700 removes), so each bucket needs at least four shards, and a leaderboard GSI with one partition key needs at least four as well.
- 42,000 writes a second exceeds the default 40,000-unit table quota, so 1,000,000 Players needs a quota increase.
- Frankfurt prices were not readable on the pricing pages; `eu-central-1` usually costs more than `us-east-1` _(not verified)_.

#### Firestore

Verified:

- Standard edition: 0.09 USD per 100,000 document writes and 0.03 USD per 100,000 reads in `us-central1`, with 20,000 free writes and 50,000 free reads a day ([Firestore pricing](https://firebase.google.com/docs/firestore/enterprise/pricing), [billing](https://firebase.google.com/docs/firestore/pricing)).

_(inference)_:

| Concurrent Players | Writes a second | Firestore a month |
| --- | --- | --- |
| 1,000 | 40 | 93 USD |
| 10,000 | 420 | 980 USD |
| 100,000 | 4,200 | 9,800 USD |
| 1,000,000 | 42,000 | 98,000 USD |

- The 500 writes a second cap on monotonically increasing indexed fields is reached at about 30,000 Players on the Guesses collection, before cost becomes the issue.

#### Fixed-price SQL for comparison

- Render Postgres: 1 CPU and 4 GB for 55 USD a month, 2 CPU and 16 GB for 160 USD, storage 0.30 USD per GB ([Render pricing](https://render.com/pricing), read from the search index because the page did not render; check before relying on it).

### 3. Other needs

| Need | SQLite / Postgres | DynamoDB | Firestore |
| --- | --- | --- | --- |
| Start-up rebuild by due time | One query on a partial index; 10,000 rows in about 2 ms _(measured)_ | Sparse GSI queried bucket by bucket, eventually consistent, written on every place and resolve | Composite index on a monotonically increasing field, capped at 500 writes a second |
| Leaderboard by Score | Index on `score`; top N reads N index entries; a Player's rank is a count over the index _(inference)_ | GSI on Score; a single partition key needs write sharding past 1,000 units a second and a merge to read the top N; no rank operation _(inference)_ | Ordered query with limit; per-document limits favour write-time aggregation _(inference)_ |
| Accounts with sign-in methods | A unique index on (provider, subject) plus a foreign key to the Player | One item per sign-in identity with `attribute_not_exists`, written in a transaction with the Player _(inference)_ | Same as DynamoDB, in a transaction _(inference)_ |
| Ad-hoc analysis | Any SQL query, or a read replica | Export to S3 and query there; the export quota is documented ([quotas](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/ServiceQuotas.html)) | Not investigated |
| Migrations | sqlx embedded migrations checked at compile time (ADR 0006) | Schemaless items; a new access pattern means a new GSI, whose backfill can throttle the base table ([GSI](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/GSI.html)) | Schemaless; new composite indexes |
| Shared store-port test suite | SQLite in-process; Postgres in a container, same suite _(inference)_ | DynamoDB local, which ignores capacity, never throws `TransactionConflictException`, and makes reads appear strongly consistent ([DynamoDB local](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/DynamoDBLocal.UsageNotes.html)) | Emulator, not investigated |
| Operational burden | SQLite: one file on one disk, no backup today (ADR 0003). Postgres: a managed instance, HA from 1 CPU on Render | No servers; capacity mode, GSIs, IAM and quotas to manage | No servers; index exemptions and ramp-up rules |

- Redis's sorted sets give the best leaderboard of all: rank and top N in O(log N), updated by `ZINCRBY` on each resolution, rebuildable from the SQL store if lost _(inference)_.
- DynamoDB local passing the shared suite proves less than SQLite passing it, because the conflicts and stale GSI reads that production can show never happen locally _(inference)_.

### 4. Verdict

Keep SQL.
SQLite stays right while one process is enough; Postgres is the next step behind the same store port; neither needs replacing within ADR 0003's table up to 100,000 Players.

Thresholds, concretely:

- **Up to about 10,000 Players: SQLite.** The single writer is not the limit if placed Guesses are grouped into a transaction, as resolutions already are, or if `synchronous=NORMAL` is accepted.
  It ends when the game needs a second instance or no single disk to lose, which ADR 0003 already puts at about 10,000 Players.
- **About 10,000 to 100,000 Players and beyond: Postgres, one primary.** 4,200 writes a second is a modest load for one instance with group commit.
  Add high availability for deploys and failures, monthly partitions of the Guess table once it outgrows memory, and Redis for a leaderboard if one comes.
  Read replicas are for analysis, not for the game.
- **About 1,000,000 Players: shard by Player.** Citus or Postgres per partition keeps SQL, and DynamoDB with provisioned, reserved capacity becomes a fair alternative at about 9,000 to 20,000 USD a month for the base table, plus sharded GSIs.
  Which wins depends on the measured cost of the Postgres cluster and on whether the leaderboard and rebuild queries move elsewhere, which this research did not settle.
- **At any scale, if the server becomes serverless:** DynamoDB, because a function has no long-lived process to hold a connection pool _(inference)_ and the demo-scale cost is about 0 USD ("AWS serverless for the game").

Access-pattern changes that would favour NoSQL, and how likely they are:

- **The always-on process goes away** (serverless): favours DynamoDB.
  This is the only realistic one.
- **Multi-region writes with low latency for Players worldwide**: would favour DynamoDB global tables, but ADR 0002 resolves every Guess against one Price source and one server clock, so writes in several regions would not help the rules _(inference)_.
- **Writes without conditions** (for example, a log of every price or every client event): favours Cassandra, ScyllaDB or DynamoDB; today the game stores no prices, and every write is conditional.

The planned changes (accounts, leaderboards, analysis) all favour SQL.

## Local measurements

Both runs used throwaway scripts outside the repository, on 2026-10-31, on an Apple M5 laptop.

- **SQLite:** Python's built-in `sqlite3` (SQLite 3.50.4), WAL mode, schema and statements as in section 1, 200,000 Players, 20,000 placed Guesses per run (5,000 for `fullfsync` with `FULL`), half of them resolved in batches of 1, 100 and 1,000.
  Each placement was its own `BEGIN IMMEDIATE ... COMMIT`.
- **Postgres:** the `postgres:17` image in Docker with default settings, 1,000,000 Players, `pgbench -T 20` with a script of two transactions: an `INSERT ... ON CONFLICT (player_id) WHERE resolved_at IS NULL DO NOTHING`, then a CTE that resolves the Open guess and adds to the Score.
  No transaction failed.

Neither run models a cloud volume's flush latency, network round trips, or the sqlx driver; they show relative costs (per-commit flush versus batching, concurrency and group commit), not production capacity.

## Not investigated

- Frankfurt prices for DynamoDB and Firestore, and Render's Postgres prices from a readable page.
- Benchmarks of managed Postgres (RDS, Cloud SQL, Render) for this workload; no first-party writes-a-second figure exists, only IOPS.
- The Firestore emulator's fidelity, ad-hoc analysis on Firestore, and ScyllaDB's LWT performance in detail.
- Cassandra's Accord transactions, which may change the Cassandra row once released.
- Litestream or another replication of SQLite to object storage, which ADR 0003 names for backups.
- How large a Resolved guess row is in practice, and whether the game keeps every Resolved guess at all; the storage estimates assume about 100 bytes and full history.
