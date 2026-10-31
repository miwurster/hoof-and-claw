# Best practices and de facto standards for a modern Rust server project

Research for [#16](https://github.com/miwurster/hoof-and-claw/issues/16), part of the map [#1](https://github.com/miwurster/hoof-and-claw/issues/1).
All sources were read on 2026-10-31.

## Question

What are the current best practices and de facto standards for a Rust server of this shape: one always-on async process serving HTTP, SSE and a static React build, holding an outbound WebSocket feed to the Price source, with SQLite behind a store port and a ports-and-adapters domain core, written mostly by agents behind strict guardrails?

## Answer

Most of the pending candidate answers match the evidence, and the stack is the mainstream one in 2026: axum on tokio with tower-http, tokio-tungstenite with rustls, thiserror plus anyhow, tracing, cargo-deny, cargo-nextest and proptest.
Three candidates need adjusting.

- **Database: keep sqlx, but it is a closer call than it looks.**
  sqlx with `query!` and committed `.sqlx` metadata is the stronger guardrail for agent-written SQL.
  rusqlite is equally popular and fits the single-writer design more directly.
  Its scale-path advantage is small, because DynamoDB is not SQL and a Postgres adapter is new code either way.
  The real guardrail is a contract test suite for the store port that every adapter must pass.
- **Lints: enforce `#[expect(reason)]` with Clippy, and allow `unwrap`/`expect`/`panic` in tests through `clippy.toml`.**
  Clippy's `allow_attributes` and `allow_attributes_without_reason` lints turn the convention into a check.
  `pedantic` needs `priority = -1` in `[workspace.lints]`, or single-lint overrides do not apply.
- **Unused dependencies: prefer cargo-shear to cargo-machete.**
  cargo-machete calls itself "fast (yet imprecise)" and is more widely installed.
  cargo-shear parses the code and also reports misplaced dependencies and source files no module tree reaches, which agents tend to leave behind.

Things the candidate list did not cover, with a recommendation each:

- Pin the toolchain to an exact version, today `1.99.1`, set `rust-version` to the same value, and let Dependabot's `rust-toolchain` ecosystem propose bumps.
- Lay the workspace out flat under `crates/`, with the domain core as its own crate that depends on no async runtime or I/O crate, and enforce the direction with Cargo itself, cargo-deny `wrappers`, and one architecture test over `cargo metadata`.
- Read configuration from environment variables once at start-up into a typed struct, with `clap` derive and its `env` feature; Render sets `PORT`, default `10000`.
- Build a glibc binary with cargo-chef in a multi-stage Dockerfile and run it on `gcr.io/distroless/cc-debian13:nonroot`; skip musl unless a reason appears.
- Cache CI with `Swatinem/rust-cache@v2` and the Docker build with BuildKit's `type=gha` cache; sccache is not needed at this size.

## Legend

- **Verified**: read on the owner's page, repository or crates.io API on 2026-10-31.
- **Inferred**: my conclusion from verified facts, not stated by the owner.
- **Recent downloads** are crates.io's `recent_downloads`, the last 90 days.

## Adoption snapshot

Numbers from the crates.io API on 2026-10-31; "Released" is the date of the newest stable version.

| Area | Crate | Newest | Released | Recent downloads | MSRV |
| --- | --- | --- | --- | --- | --- |
| HTTP | axum | 0.8.9 | 2026-04-14 | 148,484,454 | 1.80 |
| HTTP | actix-web | 4.18.1 | 2026-10-23 | 12,869,071 | 1.89 |
| HTTP | warp | 0.4.3 | 2026-05-04 | 6,702,697 | none |
| HTTP | rocket | 0.5.2 | 2026-10-23 | 2,272,426 | 1.89 |
| HTTP | salvo | 1.0.1 | 2026-10-04 | 1,012,556 | 1.94 |
| HTTP | poem | 3.1.12 | 2025-07-28 | 890,363 | 1.85 |
| HTTP | tower-http | 0.7.1 | 2026-08-31 | 199,279,739 | 1.65 |
| Runtime | tokio | 1.54.1 | 2026-10-21 | 284,588,304 | 1.85 |
| Database | sqlx | 0.9.0 | 2026-05-21 | 52,707,679 | 1.94.0 |
| Database | rusqlite | 0.41.1 | 2026-10-28 | 52,889,148 | 1.89.0 |
| Database | diesel | 2.3.14 | 2026-10-07 | 10,980,473 | 1.86.0 |
| Database | sea-orm | 2.0.4 | 2026-09-27 | 5,753,909 | 1.94.0 |
| Database | tokio-rusqlite | 0.8.0 | 2026-09-06 | 940,384 | none |
| Database | libsql | 0.10.0 | 2026-10-13 | 845,135 | none |
| Database | turso | 0.8.3 | 2026-10-12 | 754,785 | none |
| Migrations | refinery | 0.10.0 | 2026-10-02 | 3,165,417 | 1.92 |
| Migrations | rusqlite_migration | 2.6.0 | 2026-05-28 | 1,179,298 | 1.95 |
| WebSocket | tokio-tungstenite | 0.30.0 | 2026-07-11 | 103,881,267 | 1.85 |
| WebSocket | tokio-websockets | 0.13.3 | 2026-06-25 | 8,743,402 | 1.89 |
| WebSocket | fastwebsockets | 0.10.0 | 2025-01-14 | 1,925,399 | none |
| TLS | rustls | 0.23.46 | 2026-10-14 | 260,528,194 | 1.71 |
| TLS | native-tls | 0.2.18 | 2026-02-18 | 67,407,426 | 1.80 |
| Errors | thiserror | 2.0.21 | 2026-09-23 | 480,216,398 | 1.77 |
| Errors | anyhow | 1.0.104 | 2026-07-18 | 267,803,033 | 1.68 |
| Errors | eyre | 0.6.14 | 2026-08-11 | 25,322,356 | 1.65.0 |
| Errors | snafu | 0.9.2 | 2026-07-21 | 24,904,971 | 1.65 |
| Errors | miette | 7.6.0 | 2025-04-27 | 23,501,556 | 1.70.0 |
| Config | clap | 4.6.7 | 2026-09-14 | 273,653,285 | 1.85 |
| Config | config | 0.15.27 | 2026-09-30 | 21,817,820 | 1.88.0 |
| Config | figment | 0.10.19 | 2024-05-17 | 13,292,821 | none |
| Config | envy | 0.4.2 | 2021-01-04 | 4,484,240 | none |
| Config | dotenvy | 0.15.7 | 2023-03-22 | 61,694,836 | 1.56.1 |
| Logging | tracing | 0.1.44 | 2025-12-18 | 248,016,586 | 1.65.0 |
| Logging | tracing-subscriber | 0.3.23 | 2026-03-13 | 178,887,715 | 1.65.0 |
| Logging | tracing-opentelemetry | 0.34.0 | 2026-09-23 | 58,496,657 | 1.75.0 |
| Tests | proptest | 1.12.0 | 2026-10-10 | 67,275,599 | 1.88 |
| Tests | quickcheck | 1.1.0 | 2026-02-10 | 14,660,381 | 1.85.0 |
| Tests | insta | 1.49.0 | 2026-10-03 | 36,229,908 | 1.66.0 |
| Tests | rstest | 0.28.0 | 2026-10-18 | 35,503,541 | 1.85.0 |
| Tests | mockall | 0.15.0 | 2026-06-28 | 33,586,774 | 1.77.0 |
| Tools | cargo-nextest | 0.9.150 | 2026-10-26 | 1,841,151 | 1.91 |
| Tools | cargo-audit | 0.22.2 | 2026-06-05 | 4,149,047 | 1.88 |
| Tools | cargo-deny | 0.20.2 | 2026-07-09 | 2,169,242 | 1.88.0 |
| Tools | cargo-llvm-cov | 0.9.1 | 2026-09-06 | 1,928,969 | 1.87 |
| Tools | cargo-chef | 0.1.78 | 2026-08-12 | 1,268,696 | none |
| Tools | cargo-machete | 0.9.2 | 2026-04-15 | 613,407 | none |
| Tools | cargo-mutants | 27.1.0 | 2026-06-02 | 395,312 | 1.88 |
| Tools | sccache | 0.19.0 | 2026-10-24 | 137,977 | 1.91.0 |
| Tools | cargo-udeps | 0.1.61 | 2026-04-29 | 109,687 | none |
| Tools | cargo-shear | 1.14.0 | 2026-09-22 | 64,353 | 1.95 |

Download counts for tools installed as binaries undercount their use, because CI usually installs prebuilt binaries rather than from crates.io.

## HTTP framework and async runtime

**Candidate: axum on tokio with tower-http. Evidence agrees.**

- Verified: axum has about 11 times the recent downloads of actix-web, the next framework, and is maintained in the `tokio-rs` organization ([crates.io](https://crates.io/crates/axum), [repository](https://github.com/tokio-rs/axum)).
- Verified: axum's newest release is 0.8.9 from 2026-04-14, and its changelog has an unreleased section with breaking changes, among them `axum::serve` applying hyper's default `header_read_timeout` and `tower-log` leaving the default features ([changelog](https://github.com/tokio-rs/axum/blob/main/axum/CHANGELOG.md)).
- Inferred: a 0.9 release is likely during the project, so expect one small migration.
- Verified: tower-http 0.7.1 (2026-08-31) provides `ServeDir`, trace, compression, timeout, CORS and request-id layers; 0.7.1 changed `ServeDir::try_call` to propagate I/O errors instead of turning them into `404` when no fallback is set ([changelog](https://github.com/tower-rs/tower-http/blob/main/tower-http/CHANGELOG.md)).
- Inferred: serve the React build with `ServeDir` and a fallback to `index.html`, so client routes resolve, and give hashed assets a long `Cache-Control`.
- Inferred: axum's built-in `axum::response::sse` with a keep-alive covers the SSE heartbeats ADR 0003 asks for, and a `tokio::sync::broadcast` channel fans one price out to every stream.
- Verified: tokio 1.54.1 (2026-10-21) is the runtime under axum, hyper, sqlx and tokio-tungstenite, and names LTS minors: `1.51.x` until March 2027 and `1.53.x` until September 2027 ([README](https://github.com/tokio-rs/tokio#supported-rust-versions)).
- Inferred: an application with a pinned lockfile does not need an LTS minor; take current tokio through the dependency bot.

Alternatives: actix-web is mature and maintained but has its own actor model and a smaller middleware ecosystem than tower.
poem has had no release since 2025-07, warp and rocket have slow release cadences, and salvo has a small user base.

## Database access and migrations

**Candidate: sqlx with `query!`, committed `.sqlx` metadata and sqlx migrations, against rusqlite. Evidence supports sqlx, with caveats; rusqlite is a credible equal.**

### sqlx

- Verified: sqlx 0.9.0 was released on 2026-05-06 (published 2026-05-21) and moved from `launchbadge/sqlx` to the [`transact-rs`](https://github.com/transact-rs/sqlx) organization, because LaunchBadge no longer owned or maintained it ([changelog](https://github.com/transact-rs/sqlx/blob/main/CHANGELOG.md)).
- Verified: 0.9 sets its MSRV to Rust 1.94.0, following a policy of "the second-to-latest stable release as of the beginning of the current release cycle" ([FAQ](https://github.com/transact-rs/sqlx/blob/main/FAQ.md#MSRV)).
- Verified: 0.9 adds a per-crate `sqlx.toml` (behind the `sqlx-toml` feature), makes `query*()` functions take `impl SqlSafeStr` so a `format!`-built query needs an explicit `AssertSqlSafe`, and lets `libsqlite3-sys` float within a range ([changelog](https://github.com/transact-rs/sqlx/blob/main/CHANGELOG.md)).
- Inferred: `SqlSafeStr` is a useful guardrail against agents formatting values into SQL.
- Verified: the macros check SQL against a live database at compile time; offline mode reads saved query data from `.sqlx`, which `cargo sqlx prepare` must refresh before every commit that changes a query ([README](https://github.com/transact-rs/sqlx), [FAQ](https://github.com/transact-rs/sqlx/blob/main/FAQ.md)).
- Verified: for SQLite, the macros infer nullability by stepping through the prepared statement's bytecode, and the FAQ calls nullability inference imperfect, erring towards marking columns nullable ([FAQ](https://github.com/transact-rs/sqlx/blob/main/FAQ.md)).
- Verified: each sqlx SQLite connection runs on its own dedicated thread ([source](https://github.com/transact-rs/sqlx/blob/main/sqlx-sqlite/src/connection/worker.rs)).
- Verified: the `sqlite` feature bundles and statically links SQLite, and sqlx is `#![forbid(unsafe_code)]` except with that feature ([README](https://github.com/transact-rs/sqlx)).
- Verified: sqlx removed `Cargo.lock` from its repository in 0.9, so `cargo install --locked sqlx-cli` no longer works ([changelog](https://github.com/transact-rs/sqlx/blob/main/CHANGELOG.md)).

How to use it here (Inferred):

- A writer pool with `max_connections(1)` and a separate read pool give the single writer from ADR 0003; set WAL, `synchronous = NORMAL` and a busy timeout in `SqliteConnectOptions`.
- CI runs `cargo sqlx prepare --workspace --check` and builds with `SQLX_OFFLINE=true`, so a stale `.sqlx` fails the Green gate instead of the Docker build.
- Pin `sqlx-cli` to the same version as `sqlx` in CI, and install it with `cargo install --version`, since `--locked` no longer works.
- Embed migrations with `sqlx::migrate!` and run them at start-up, before the server accepts requests.

### rusqlite

- Verified: rusqlite 0.41.1 (2026-10-28) has the same recent downloads as sqlx, recommends the `bundled` feature for programs that control their own database, and is a synchronous wrapper ([README](https://github.com/rusqlite/rusqlite)).
- Inferred: a dedicated writer thread fed by a channel is the most literal form of ADR 0003's single writer, and `tokio-rusqlite` packages that pattern.
- Inferred: rusqlite has no compile-time query checks, so SQL mistakes surface only when the adapter's integration tests run; with agents writing the SQL, that moves the failure from compile to test.
- Migrations with rusqlite come from `rusqlite_migration` or `refinery`.

### Others

- diesel 2.3 supports SQLite through a typed query DSL, but diesel-async reaches SQLite only through a wrapper around the sync connection ([diesel_async README](https://github.com/diesel-rs/diesel_async)); a typed DSL adds a second language for agents to learn.
- sea-orm is an ORM on top of sqlx; the store port follows game access patterns, not entities, so an ORM adds a layer without a use.
- turso is a Rust rewrite of SQLite that its README says runs in production at several organizations; libsql is Turso's older fork ([README](https://github.com/tursodatabase/turso)).
  Both have under a million recent downloads, and turso is pre-1.0, so neither is a conservative choice for the demo.

### Verdict

Keep sqlx.
Its compile-time checks and `SqlSafeStr` catch mistakes in agent-written SQL before tests run, and it is async-native.
The scale-path argument for sqlx is weaker than it seems, because SQL dialects differ, `query!` is database-specific, and DynamoDB is not SQL, so each new store is a new adapter.
Whichever library is used, write one contract test suite against the store port and run it against every adapter; that suite, not the library, is what keeps the swap safe.

## WebSocket client and TLS

**Candidate: tokio-tungstenite with rustls. Evidence agrees.**

- Verified: tokio-tungstenite has about 12 times the recent downloads of tokio-websockets, the next client, and axum's own WebSocket support builds on it ([crates.io](https://crates.io/crates/tokio-tungstenite)).
- Verified: no TLS is enabled by default; `wss://` needs one of `native-tls`, `rustls-tls-native-roots` or `rustls-tls-webpki-roots` ([README](https://github.com/snapview/tokio-tungstenite)).
- Verified: tokio-tungstenite depends on rustls with `default-features = false`, and its README warns about a panic with rustls features since 0.23 that comes from no crypto provider being chosen ([Cargo.toml](https://github.com/snapview/tokio-tungstenite/blob/master/Cargo.toml), [issue 336](https://github.com/snapview/tokio-tungstenite/issues/336)).
- Inferred: add `rustls` as a direct dependency with one provider feature, and install that provider as the process default at start-up, before the feed connects.
- Verified: the rustls team recommends the aws-lc-rs provider for features and performance, and calls ring easier to build on more platforms ([rustls README](https://github.com/rustls/rustls)).
- Inferred: `rustls-tls-webpki-roots` bundles Mozilla's roots in the binary, so the image needs no CA bundle; native roots would follow the base image's CA updates instead.
- Inferred: a cargo-deny `use-instead` ban on `openssl` keeps native-tls from creeping back in through a feature.
- Inferred: no client library reconnects for you; the Price source adapter needs its own loop with capped, jittered backoff and a dead-feed timeout.

Alternatives: tokio-websockets is a fast, maintained client and server with rustls support, but much less used.
fastwebsockets has had no release since 2025-01.

## Toolchain pinning, edition and MSRV

**Candidate: `rust-toolchain.toml` pinning an exact stable version, edition 2024. Evidence agrees.**

- Verified: the current stable release is Rust 1.99.1 of 2026-10-13 ([channel manifest](https://static.rust-lang.org/dist/channel-rust-stable.toml)).
- Verified: edition 2024 is stable since Rust 1.85 and makes Cargo's resolver `3`, which prefers dependency versions compatible with `rust-version` ([edition guide](https://doc.rust-lang.org/edition-guide/rust-2024/cargo-resolver.html)).
- Verified: rustup 1.28.0 stopped installing the active toolchain automatically and 1.28.1 restored it by default; `rustup toolchain install` without arguments installs the toolchain named in `rust-toolchain.toml` ([rustup changelog](https://github.com/rust-lang/rustup/blob/main/CHANGELOG.md)).
- Verified: Cargo's Rust version chapter says the simplest policy is "always use the latest Rust version", and asks that `rust-version` follow a stated policy rather than drift ([Cargo book](https://doc.rust-lang.org/cargo/reference/rust-version.html)).
- Verified: Dependabot updates `rust-toolchain.toml` through the `rust-toolchain` ecosystem since 2025-08-19, for version updates only ([GitHub changelog](https://github.blog/changelog/2025-08-19-dependabot-now-supports-rust-toolchain-updates/)).

Policy for this application (Inferred):

- Pin `channel = "1.99.1"` with `components = ["rustfmt", "clippy"]` and `profile = "minimal"`.
- Set `rust-version = "1.99"` in `[workspace.package]`, so resolver 3 and Clippy's `incompatible_msrv` agree with the pinned toolchain.
- An application has no downstream users, so its MSRV is simply the pinned version; bump both on each stable release through the bot's pull request, which runs the Green gate.
- An exact pin matters most for Clippy, because each release adds lints; with `pedantic` denied, a floating `stable` would break the gate on release day.
- Keep the Docker builder image on the same version, for example `rust:1.99.1-slim-trixie`, and let Dependabot's `docker` ecosystem bump it alongside.

## Workspace and crate layout

**Not a candidate yet. Recommendation: a virtual workspace, flat under `crates/`, with the domain core as its own crate.**

- Verified: a Cargo package can only use crates it declares, and Cargo rejects dependency cycles, so a crate boundary is a dependency rule the compiler enforces ([Cargo book, workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)).
- Verified: matklad, of rust-analyzer, recommends a virtual manifest at the root and all crates in a flat `crates/` directory, named after their folder ([Large Rust Workspaces](https://matklad.github.io/2021/08/22/large-rust-workspaces.html)).
- Verified: there is no official ArchUnit equivalent; `arch_test_core` has had no release since 2021, and a Rust users forum thread reaches the same conclusion ([forum](https://users.rust-lang.org/t/archunit-equivalent-for-rust-arch-unit-rs/140626)).
- Verified: cargo-deny's `bans.deny` takes `wrappers`, which allows only the listed crates a direct dependency on a banned crate ([cargo-deny docs](https://github.com/EmbarkStudios/cargo-deny/blob/main/docs/src/checks/bans/cfg.md)).

A layout that fits the ADRs (Inferred):

| Crate | Holds | May depend on |
| --- | --- | --- |
| `domain` | Player, Guess, Score, resolution rules, port traits | `thiserror`, pure crates; no tokio, sqlx, axum, tungstenite |
| `store-sqlite` | store port adapter, migrations | `domain`, `sqlx` |
| `price-bitstamp` | Price source adapter | `domain`, `tokio-tungstenite`, `rustls` |
| `http` | axum routes, SSE, static files | `domain`, `axum`, `tower-http` |
| `server` | binary: configuration, wiring, start-up | every crate above |
| `acceptance` | end-to-end tests against a running server | `reqwest`, test-only crates |

How to enforce the direction (Inferred):

- Put `[workspace.dependencies]` and `[workspace.lints]` in the root manifest, and give every member `lints.workspace = true`.
- Add cargo-deny `wrappers` for `tokio`, `sqlx`, `axum` and `tokio-tungstenite`, so only the intended adapters may depend on them directly.
- Add one architecture test that reads `cargo metadata` (the `cargo_metadata` crate) and asserts each crate's dependencies against an allowlist; it fails with a message an agent can act on.
- Native `async fn` in traits is stable since Rust 1.75 but is not dyn-compatible; use generics for ports, or the `trait-variant` crate when a port must be `dyn` with a `Send` bound ([Rust blog](https://blog.rust-lang.org/2023/12/21/async-fn-rpit-in-traits/)).
- Keep the domain synchronous where it can be.
  Resolution on a price is a pure function over the in-memory index, and only the adapters and the application loop await.

## Lints and formatting

**Candidate: workspace `[lints]` with `unsafe_code = "forbid"`, Clippy `pedantic` denied with `#[expect(reason)]`, and `unwrap_used`, `expect_used`, `panic`, `todo`, `dbg_macro` denied outside tests; rustfmt. Evidence agrees, with three refinements.**

- Verified: `[lints]` takes a `priority`; lower numbers are overridden by higher ones, so a group such as `pedantic` needs `priority = -1` for single-lint entries to override it ([Cargo book, manifest](https://doc.rust-lang.org/cargo/reference/manifest.html#the-lints-section)).
- Verified: Cargo applies `[lints]` only to the current package, never to dependencies, so `unsafe_code = "forbid"` does not conflict with sqlx or rustls using unsafe ([Cargo book, manifest](https://doc.rust-lang.org/cargo/reference/manifest.html#the-lints-section)).
- Verified: Clippy's book says `pedantic` lints "are rather strict or have occasional false positives", and that `restriction` "should, *emphatically*, not be enabled as a whole" but picked lint by lint ([Clippy book](https://doc.rust-lang.org/clippy/)).
- Verified: `clippy.toml` has `allow-unwrap-in-tests`, `allow-expect-in-tests`, `allow-panic-in-tests`, `allow-dbg-in-tests` and `allow-print-in-tests`, all `false` by default, each covering test functions and `#[cfg(test)]` ([Clippy lint configuration](https://doc.rust-lang.org/clippy/lint_configuration.html)).
- Verified: Rust 1.81 stabilized the `expect` lint level, which warns when the expected lint does not occur, and lint attributes take a `reason` since the same release ([Rust 1.81 announcement](https://blog.rust-lang.org/2024/09/05/Rust-1.81.0/)).

Refinements (Inferred):

- Deny `clippy::allow_attributes` and `clippy::allow_attributes_without_reason`, so `#[expect(..., reason = "...")]` is checked rather than a convention.
  With warnings denied, an `#[expect]` that is no longer needed then fails the gate, so suppressions cannot go stale.
- Use the `clippy.toml` `allow-*-in-tests` keys instead of `#[cfg_attr(test, allow(...))]` on every test module.
- Add `unimplemented`, `print_stdout`, `print_stderr` (logging goes through tracing) and `indexing_slicing` to the denied restriction lints, and leave `nursery` and `cargo` off.
- Run `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` in CI, rather than `RUSTFLAGS="-D warnings"`, which would invalidate the build cache for every other step.
- Denying `expect_used` everywhere outside tests can push agents to `unwrap_or_default` that hides a bug; review for that pattern, and allow `expect` with a reason in start-up code where a crash is the right outcome.
- rustfmt needs little configuration: `cargo fmt` takes its style edition from the package edition, and `cargo fmt --all --check` runs in the gate.

## Supply-chain checks and unused dependencies

**Candidates: cargo-deny and cargo-machete. Keep cargo-deny; prefer cargo-shear.**

- Verified: cargo-deny 0.20.2 (2026-07-09) checks RustSec advisories, licenses, bans and sources in one tool, which covers cargo-audit's advisory check ([cargo-deny docs](https://github.com/EmbarkStudios/cargo-deny)).
- Inferred: use the bans check for the architecture `wrappers` above and for `openssl`, and deny unknown registries and git sources.
- Verified: cargo-machete describes itself as detecting unused dependencies "in a fast (yet imprecise) way" ([README](https://github.com/bnjbvr/cargo-machete)).
- Verified: cargo-shear parses code with rust-analyzer's parser and reports unused dependencies, misplaced dev or build dependencies, and source files that no module tree reaches; it cannot see imports hidden in macro expansions without `--expand` on nightly ([README](https://github.com/Boshen/cargo-shear)).
- Verified: cargo-machete has about ten times cargo-shear's recent downloads ([crates.io](https://crates.io/crates/cargo-machete)).
- Inferred: cargo-shear's unlinked-file check catches a typical agent mistake, a new file never added to `mod`, so `cargo shear --deny-warnings` is the better gate; cargo-machete remains an acceptable fallback.
- cargo-udeps is the precise option but needs nightly, which conflicts with a pinned stable toolchain.

## Error handling

**Not a candidate yet. Recommendation: thiserror in the domain and adapters, anyhow only in the binary.**

- Verified: thiserror and anyhow are the two most downloaded error crates by a wide margin, both by the same author ([thiserror](https://crates.io/crates/thiserror), [anyhow](https://crates.io/crates/anyhow)).
- Verified: the Rust API Guidelines ask error types to implement `std::error::Error` and be `Send + Sync` and meaningful ([C-GOOD-ERR](https://rust-lang.github.io/api-guidelines/interoperability.html#c-good-err)).
- Inferred: port errors are part of the port's contract, so they are typed enums in `domain`; adapters map their library errors into them, and the `http` crate maps them to status codes through `IntoResponse`.
- Inferred: anyhow belongs in `server` for start-up failures, where the only handling is to log and exit.
- Alternatives: snafu suits large codebases that want context selectors; error-stack and miette target rich diagnostics; eyre and color-eyre target CLIs.

## Configuration

**Not a candidate yet. Recommendation: environment variables, parsed once at start-up into a typed struct with `clap` derive and its `env` feature.**

- Verified: Render recommends binding to the `PORT` variable, defaults it to `10000`, and requires binding to `0.0.0.0` ([Render docs](https://render.com/docs/web-services)).
- Verified: figment's last release is from 2024-05, envy's from 2021-01 and dotenvy's from 2023-03, while clap and config released in 2026-09 ([crates.io](https://crates.io/crates/clap)).
- Inferred: clap gives a typed struct, defaults, validation and `--help` from one derive, and it is already the most maintained option; the `config` crate is the choice if layered files become necessary.
- Inferred: fail fast at start-up on a missing or malformed variable, log the effective configuration without secrets, and pass the struct into the composition root rather than reading the environment elsewhere.

## Logging and tracing

**Not a candidate yet. Recommendation: `tracing` with `tracing-subscriber`, JSON to stdout, and tower-http's `TraceLayer`.**

- Verified: tracing is maintained in the `tokio-rs` organization and is the instrumentation that tokio, hyper, axum, tower-http and sqlx emit ([tracing](https://github.com/tokio-rs/tracing)).
- Inferred: use `tracing-subscriber`'s `fmt` layer with JSON output and an `EnvFilter` from `RUST_LOG`, because Render collects stdout.
- Inferred: give every Guess a span with its id, so placing and resolving it can be followed across the feed and HTTP; log every received price at `debug` only.
- Inferred: add `tracing-opentelemetry` later, on the AWS step, without touching call sites.

## Unit, property and acceptance tests

**Candidate: cargo-nextest and proptest. Evidence agrees.**

- Verified: nextest does not run doctests, so a separate `cargo test --doc` is needed if the code has any ([nextest docs](https://nexte.st/docs/integrations/test-coverage/)).
- Verified: proptest 1.12.0 (2026-10-10) has about five times quickcheck's recent downloads and shrinks failing inputs through strategies ([crates.io](https://crates.io/crates/proptest)).
- Inferred: property tests fit the resolution rules, for example that a Guess never resolves before 60 seconds, never on an unchanged price, and the Score always equals Correct minus Wrong.
- Inferred: hand-written fakes for the ports, kept in the `domain` crate behind a `test-support` feature, are clearer than mockall and double as the reference for the store contract suite.
- Inferred: tokio's `test-util` feature with `tokio::time::pause` drives the 60-second rule deterministically in async tests; the Clock port does the same for the domain.
- Inferred: run acceptance tests over real HTTP against the started server with `reqwest`, including an SSE stream; `tower::ServiceExt::oneshot` is fine for handler tests.
- Optional: insta for snapshotting API responses, cargo-llvm-cov for coverage, and cargo-mutants to check that the tests catch changed behaviour.

## Build and Docker image

**Candidate: cargo-chef and a distroless or static musl image. Recommendation: cargo-chef, a glibc build, and distroless `cc`.**

- Verified: cargo-chef splits a build into `prepare` (a recipe of manifests and the lockfile) and `cook` (build dependencies only), so the dependency layer is cached until the recipe changes; `cook` and `build` must run from the same working directory ([README](https://github.com/LukeMathWalker/cargo-chef)).
- Verified: distroless images are based on Debian 13 and include `static-debian13` (about 2 MiB) and `cc-debian13` (with glibc and libgcc), each with a `nonroot` tag; the distroless Rust example uses `cc` ([distroless README](https://github.com/GoogleContainerTools/distroless), [Rust example](https://github.com/GoogleContainerTools/distroless/blob/main/examples/rust/Dockerfile)).
- Verified: Render disks are available only at runtime, and a disk disables zero-downtime deploys ([Render docs](https://render.com/docs/disks)).
- Inferred: a glibc build on `rust:1.99.1-slim-trixie` with `gcr.io/distroless/cc-debian13:nonroot` avoids musl's cross-compilation of aws-lc-rs and the bundled SQLite, and avoids musl's slower allocator under many threads.
- Inferred: choose the static musl route only if image size starts to matter, and then pair it with the ring provider and a faster allocator.
- Inferred: build the frontend in its own Node stage and copy the static files into the runtime image next to the binary.
- Inferred: run a health endpoint over HTTP, because distroless has no shell for a command check.

## CI caching

**Candidates: Swatinem/rust-cache and sccache. Recommendation: rust-cache; sccache only if CI grows.**

- Verified: `Swatinem/rust-cache@v2`, newest release v2.9.3 of 2026-10-20, caches the Cargo registry and dependency build artifacts, keys on rustc, the lockfile, manifests and `rust-toolchain.toml`, skips workspace crates, and sets `CARGO_INCREMENTAL=0` ([README](https://github.com/Swatinem/rust-cache), [releases](https://github.com/Swatinem/rust-cache/releases)).
- Verified: GitHub limits caches to 10 GB per repository and evicts the oldest beyond that ([README](https://github.com/Swatinem/rust-cache)).
- Inferred: with one or two Rust jobs, rust-cache covers it; sccache earns its setup when many jobs or branches compile the same crates.
- Inferred: install cargo-nextest, cargo-deny, cargo-shear and sqlx-cli as prebuilt binaries, for example with `taiki-e/install-action`, rather than compiling them in every run.
- Inferred: the Docker build caches its layers with `docker/build-push-action` and `cache-from`/`cache-to: type=gha`, which is what makes the cargo-chef layer survive between runs ([Docker docs](https://docs.docker.com/build/ci/github-actions/cache/)).

## Dependency update bots

**Not a candidate yet. Recommendation: Dependabot with grouped weekly updates.**

- Verified: Dependabot covers `cargo`, `rust-toolchain`, `docker` and `github-actions` in one `dependabot.yml` ([GitHub changelog](https://github.blog/changelog/2025-08-19-dependabot-now-supports-rust-toolchain-updates/)).
- Verified: Renovate's `cargo` manager updates `Cargo.toml` and `Cargo.lock` and supports lockfile maintenance, and it has a separate Rust toolchain manager ([Renovate docs](https://docs.renovatebot.com/modules/manager/cargo/)).
- Inferred: Dependabot needs no app installation on a GitHub repository; group minor and patch updates per ecosystem so agents and the gate see one pull request a week, not twenty.
- Inferred: Renovate is the better tool if grouping or auto-merge rules outgrow Dependabot.

## Open questions

1. Render's disk documentation says nothing about file ownership, so it is unverified whether a `nonroot` distroless container can write to the mounted disk; try it on the first deploy, and fall back to a root user only for that reason.
2. axum's unreleased section holds breaking changes; whether 0.9 lands before the demo ships is unknown.
3. The `sqlx` versus `rusqlite` choice turns on how much the team values compile-time SQL checks against a simpler single-writer thread; both are well maintained, so this is a judgment call for the ADR, not a gap in evidence.
