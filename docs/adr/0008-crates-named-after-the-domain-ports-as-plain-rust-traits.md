# Crates named after the domain, ports as plain Rust traits

The server is a Cargo workspace whose crates are cut by area of the domain, not by technical layer, so the folder list reads like the game.
Each area's core is a synchronous crate that takes time and prices as values and returns decisions.
One asynchronous crate runs the game around them, and adapters named after their technology plug into ports the cores own.
Crate boundaries make the dependency direction a compiler error, which is the guardrail that replaces line-by-line review (ADR 0006).
Ports and adapters are written with Rust's ordinary features (traits, generics, enums, newtypes, modules), not with patterns carried over from Java.

## Crates

All crates sit flat under `crates/` in a virtual workspace.
The acceptance test suite is not a crate but the TypeScript package `acceptance/` (ADR 0011).

| Crate | Kind | Holds | May depend on |
| --- | --- | --- | --- |
| `prices` | core, sync | Price, Price source, the latest price, Price delayed, the `PriceSource` port | `thiserror`, pure crates |
| `guessing` | core, sync | Player, Guess, Direction, Start price, End price, Outcome, Score, resolution over the index of due guesses, its store port | `prices`, `thiserror`, pure crates |
| `identity` | core, sync | issuing and checking the anonymous token, token to Player ID, its store port | `guessing`, `thiserror`, pure crates |
| `game` | shell, async | the running game: takes prices from `PriceSource`, runs the cores, persists before it acknowledges, fans events out to the SSE streams; owns the `Clock` and `Fanout` ports | the cores, `tokio` |
| `bitstamp` | adapter | `PriceSource` over Bitstamp's WebSocket feed and REST seed (ADR 0007) | `prices`, `tokio`, `tokio-tungstenite`, `rustls` |
| `sqlite` | adapter | every core's store port on one database file and one write connection | the cores, `sqlx` |
| `web` | adapter | axum routes, the SSE stream, the identity cookie, static files | `game`, the cores, `axum`, `tower-http` |
| `server` | binary | configuration, wiring, start-up | every crate above |
| `test-controls` | test adapter | a controllable `Clock` and `PriceSource`, and the HTTP endpoints that drive them (ADR 0011) | `game`, `prices`, `axum` |

Inside a crate, modules are named after glossary terms (`guess.rs`, `score.rs`), never `models/`, `services/`, `repositories/` or `utils/`.

## Ports

There are five ports.
Each has a real adapter and a test adapter, so something varies across every seam.

- **`PriceSource` in `prices`.** It delivers a stream of received prices, each with its Price source, server receipt time and trade id, and of changes between live and Price delayed.
  Adapters: `bitstamp` and the controllable source in `test-controls`.
- **`Clock` in `game`.** It gives the server's time.
  The cores never read it.
  `game` reads it and passes the time in.
  Adapters: the system clock in `server` and the controllable clock in `test-controls`.
- **One store port per core.** `guessing`'s port stores Players, Guesses and Scores, and `identity`'s stores token hashes to Player IDs.
  Each port offers a few operations in domain language, such as placing a guess or resolving one together with its Score change in one call, never generic CRUD.
  Adapters: `sqlite` and a hand-written in-memory fake.
  Both pass one shared test suite per port (ADR 0006).
- **Identity in `identity`.** It turns a token into a Player ID, or issues a new one.
  The cookie handling around it stays in `web` (ADR 0001).

- **`Fanout` in `game`.** It publishes prices to every open SSE stream and a Player's events to that Player's streams (ADR 0010).
  Adapters: an in-process broadcast in `game` and a recording fake for `game`'s tests.
  Both pass one shared test suite.

## The first Guess across two areas

A Player is stored on the first Guess (ADR 0001), which writes to `identity` and `guessing`.
There is no transaction across the two.
The Player ID is created in memory when the cookie is issued.
On the first Guess, `game` first writes the token link, which is idempotent, then the Player and the Guess in one `guessing` transaction.
If the server dies between the two, a link without a Player is left, and it reads as Score 0 with no Open guess, as ADR 0001 already defines.

## Rust, not Java in Rust

1. A port is a plain trait named after the capability in glossary language: `PriceSource`, not `PriceSourcePort`, `IPriceSource` or `PriceSourceImpl`, and no `Service`, `Manager` or `Repository` suffix.
2. A trait exists only where two implementations exist.
   Everything else is a concrete type or a free function.
3. States are enums and identifiers and prices are newtypes, so the types cannot express an invalid state.
   A Guess is `Open` or `Resolved`, not a status string with optional fields.
4. Modules and `pub(crate)` do the encapsulation, not traits.
   There are no bags of getters and setters, and constructors check invariants.
5. Errors are `Result`s with enum error types (ADR 0006).
6. Ports use native `async fn` in traits, and `game` takes adapters as generic parameters, not as `dyn`, so neither `async-trait` nor `trait-variant` is needed.
   `main` in `server` wires concrete types with ordinary constructors; there is no dependency injection container and no `Arc<dyn …>` service locator.
7. Free functions are fine, and not everything has to be a method on a struct.

## Enforcement

All three run in the Green gate.

- **The crate graph.** One architecture test reads `cargo metadata` and checks each crate's dependencies against the "May depend on" column above, failing with a message that names the crate, the forbidden dependency and the rule.
  It lives in `crates/server/tests/architecture.rs`, because `server` already depends on every crate, and it also fails on a module named `models`, `services`, `repositories`, `utils` or `helpers`.
- **cargo-deny `wrappers`.** Only the crates named above may depend directly on `tokio`, `sqlx`, `axum`, `tower-http` and `tokio-tungstenite`.
- **Clippy.** `disallowed-methods` allows `SystemTime::now` and `Instant::now` only in the system clock adapter, and `disallowed-types` forbids `f32` and `f64` in the cores.

`test-controls` is compiled into `server` only with the Cargo feature `test-controls`.
The release image is built without it, and a check in the gate asserts that the release build does not contain the crate, so no test backdoor ships.

## Considered Options

- **Crates by layer (`domain`, `app`, `http`, `store`), as in the Rust project research.** Rejected.
  It enforces the same direction, but the structure names layers instead of the game, and every new area of the domain spreads across all of them.
- **One core crate with a module per area.** Rejected.
  It has fewer crates, but nothing enforces that `prices` does not depend on `guessing`.
- **A domain core that calls the ports itself.** Rejected.
  It makes the core async, and its tests need fakes for every rule.
  A core that takes values and returns decisions is tested with plain values.
- **One store port for everything.** Rejected.
  It is the simplest for the demo, but every new area would grow the same interface.
  When each core owns its port, it keeps a new area's storage next to its rules.
- **A transaction across `identity` and `guessing` for the first Guess.** Rejected.
  It would need a port spanning both areas, and the fixed write order leaves only a state ADR 0001 already handles.
- **A runtime flag for the test controls in the production binary.** Rejected.
  It is simpler, but it ships a way to set the price and the time to production.

## Consequences

- There are nine crates for a small game, so builds link more units; cargo-chef's cached dependency layer (ADR 0006) keeps that cost in the image build.
- The single `sqlite` crate implements every store port because there is one database file and one write connection.
  It can split per area when an area moves to another store.
- How the acceptance tests use `test-controls` is decided in ADR 0011: they send prices, lose and restore the connection, advance the clock and set it.
