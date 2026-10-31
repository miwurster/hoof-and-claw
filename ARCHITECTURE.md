# Architecture

Where code goes in this repository: workspace, crate, package, module, and what may depend on what.
Properties of code, how it is written rather than where it lives, are in `CODING_STANDARDS.md`.
The ubiquitous language lives in `GLOSSARY.md`.

A rule that names a test fails the Green gate (`mise run gate`).
The architecture tests are `crates/server/tests/architecture.rs` for the server, and the dependency-cruiser config of each TypeScript package (`spa/.dependency-cruiser.cjs`, `acceptance/.dependency-cruiser.cjs`).
A rule without a test name is convention.

## Workspaces

- A Cargo virtual workspace holds the server's crates, flat under `crates/`.
- A pnpm workspace holds the TypeScript packages: the SPA in `spa/`, the acceptance suite in `acceptance/`, and the generated API types in `contract/`.
- `mise` defines every task, the Green gate included; there is no other task runner and no Nx.

## Server: crates cut by area of the domain

Crates are named after areas of the game, not after technical layers.
Each area's core is a synchronous crate that takes time and prices as values and returns decisions.
A pure crate does no I/O, starts no async runtime and reads no clock.
`game` is the one asynchronous shell that runs the cores; adapters are named after their technology and plug into ports the cores own.

| Crate           | Kind         | Holds                                                                                                                      | May depend on                                             |
| --------------- | ------------ | -------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------- |
| `prices`        | core, sync   | Price, Price source, the latest price, Price delayed, the `PriceSource` port                                               | `thiserror`, pure crates                                  |
| `guessing`      | core, sync   | Player, Guess, Direction, Start price, End price, Outcome, Score, resolution over the index of due guesses, its store port | `prices`, `thiserror`, pure crates                        |
| `identity`      | core, sync   | issuing and checking the anonymous token, token to Player ID, its store port                                               | `guessing`, `thiserror`, pure crates                      |
| `game`          | shell, async | runs the cores on received prices, persists before it acknowledges, publishes events; owns the `Clock` and `Fanout` ports  | the cores, `tokio`                                        |
| `bitstamp`      | adapter      | `PriceSource` over Bitstamp's WebSocket feed and REST seed                                                                 | `prices`, `tokio`, `tokio-tungstenite`, `rustls`          |
| `sqlite`        | adapter      | every core's store port on one database file and one write connection                                                      | the cores, `sqlx`                                         |
| `web`           | adapter      | axum routes, the SSE stream, the identity cookie, static files, the wire types of the API contract                         | `game`, the cores, `axum`, `tower-http`, `serde`, `ts-rs` |
| `server`        | binary       | configuration, wiring, start-up                                                                                            | every crate above                                         |
| `test-controls` | test adapter | the controllable `Clock` and `PriceSource`, the HTTP endpoints that drive them, and their wire types                       | `game`, `prices`, `axum`, `serde`, `ts-rs`                |

The "May depend on" column is enforced by `crate_graph_follows_the_table` in `crates/server/tests/architecture.rs`, which reads `cargo metadata`.
Only the crates in the table may depend directly on `tokio`, `sqlx`, `axum`, `tower-http` and `tokio-tungstenite`; enforced by cargo-deny `wrappers` in `deny.toml`.

### Modules

Inside a crate, modules are named after glossary terms (`guess.rs`, `score.rs`).
No module is called `models`, `services`, `repositories`, `utils` or `helpers`; enforced by `modules_are_named_after_the_domain` in the same test file.

### Ports

| Port                    | Owned by               | Real adapter                                     | Test adapter                               |
| ----------------------- | ---------------------- | ------------------------------------------------ | ------------------------------------------ |
| `PriceSource`           | `prices`               | `bitstamp`                                       | the controllable source in `test-controls` |
| `Clock`                 | `game`                 | the system clock in `server`                     | the controllable clock in `test-controls`  |
| one store port per core | `guessing`, `identity` | `sqlite`                                         | a hand-written in-memory fake in the core  |
| `Identity`              | `identity`             | the anonymous token, its cookie handled in `web` | a hand-written in-memory fake              |
| `Fanout`                | `game`                 | the in-process broadcast in `game`               | a recording fake in `game`                 |

- A port is a plain trait named after the capability in glossary terms: `PriceSource`, never `PriceSourcePort`, `IPriceSource`, `PriceSourceImpl`, or a `Service`, `Manager` or `Repository` suffix.
- A trait exists only where two implementations exist; everything else is a concrete type or a free function.
- Every adapter of a port, fake included, passes one shared test suite for that port, kept next to the port.
- The cores never read the clock; `game` reads `Clock` and passes the time in.
  `SystemTime::now` and `Instant::now` are allowed only in the system clock adapter; enforced by Clippy `disallowed-methods` in `clippy.toml`.
- Encapsulation comes from modules and `pub(crate)`, not from traits.

### Wiring

- `game` takes its adapters as generic parameters, never as `dyn` or `Arc<dyn …>`.
- Ports use native `async fn` in traits, with no `async-trait` or `trait-variant`.
- `main` in `server` builds the concrete adapters with ordinary constructors and hands them to `game`; there is no dependency injection container.
- `anyhow` appears only in `server`.

### Test controls

`test-controls` is compiled into `server` only with the Cargo feature `test-controls`.
The release build must not contain it; enforced by `release_build_has_no_test_controls` in the Green gate.

## API contract: `contract/`

The routes, payloads and errors between the SPA and the server are fixed in ADR 0014.

- The wire types are serde types in `web`, and in `test-controls` for the test controls, separate from the domain types of the cores.
  `ts-rs` generates their TypeScript types into `contract/`.
- `contract/` holds only generated files, checked in and never edited by hand.
  The Green gate regenerates them and fails on any difference.
- Its main export holds the game's types; `contract/test-controls` holds the test controls' types.
- `spa/` imports only the main export; enforced by `spa-skips-the-test-controls-contract` in `spa/.dependency-cruiser.cjs`.
- In `acceptance/`, only `drivers/api/` imports the main export and only `drivers/controls/` imports `contract/test-controls`; enforced by `acceptance/.dependency-cruiser.cjs`.

## SPA: `spa/`

A client-rendered Vite and React app, served as static files by the game server from the same origin.

- **Routes.** TanStack Router with routes defined in code under `spa/src/routes/`; no file-based routing and no route loaders.
- **State.** One pure reducer in `spa/src/game/` takes the state and a server event or a user action and returns the new state.
  One thin hook beside it owns the `EventSource` and the `fetch` calls and feeds the reducer.
  The reducer's server events are the `ServerEvent` union from `contract/`.
  There is no data library (no TanStack Query, Redux or Zustand).
- **Containers and presentational components.** One container per route wires the hook to presentational components.
  A presentational component receives only props and callbacks and never imports the hook, `fetch` or `EventSource`; enforced by `presentational-components-skip-the-data-layer` in `spa/.dependency-cruiser.cjs`.
- **Styling.** Design tokens are CSS custom properties in one token stylesheet, named as in `docs/design/visual-direction.md`.
  Each component has its own CSS Module that refers only to tokens; Stylelint forbids raw colour values outside the token stylesheet.

## Acceptance suite: `acceptance/`

A Playwright Test package in Dave Farley's four layers, one folder each:

| Folder              | Holds                                                                            | May import                                         |
| ------------------- | -------------------------------------------------------------------------------- | -------------------------------------------------- |
| `cases/`            | test cases in glossary terms                                                     | `dsl/`                                             |
| `performance/`      | performance cases with their budgets (ADR 0014)                                  | `dsl/`                                             |
| `dsl/`              | actors (`alice`, `priceSource`, `clock`, `server`) and their `should` assertions | the driver interface                               |
| `drivers/ui/`       | the driver for the SPA in a browser                                              | Playwright                                         |
| `drivers/api/`      | the driver for the HTTP API and the SSE stream                                   | Playwright's request API, `contract/`              |
| `drivers/controls/` | calls to the `test-controls` endpoints                                           | Playwright's request API, `contract/test-controls` |

No driver imports another; enforced by `acceptance/.dependency-cruiser.cjs`.
Every case runs in the Playwright projects `ui` and `api`; a case limited to one carries that project's tag with the reason beside it.
Each worker starts its own server with `mise run launch` on port 0 and its own SQLite file.
The performance cases run in the project `performance`, with one worker and only the API driver, after the other projects (ADR 0014).

## Tests

| Layer                                      | Where                                                    | Tool                           |
| ------------------------------------------ | -------------------------------------------------------- | ------------------------------ |
| Domain rules of a core                     | the core's own tests                                     | `cargo-nextest`, `proptest`    |
| A port's contract                          | one shared suite per port, run against every adapter     | `cargo-nextest`                |
| `game`                                     | `game`'s tests with the fakes and the recording `Fanout` | `cargo-nextest`                |
| Reducer, hook, presentational components   | beside the code in `spa/`                                | Vitest, Testing Library        |
| The whole app through its public interface | `acceptance/`                                            | Playwright                     |
| Performance budgets under load (ADR 0014)  | `acceptance/performance/`, project `performance`         | Playwright, `hdr-histogram-js` |

Fakes are hand-written; there is no `mockall` and no module mocking.
