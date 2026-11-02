# Write the server in Rust on axum, guarded by the compiler and strict lints rather than human review

The server is written in Rust, on axum and tokio, with sqlx for SQLite, tokio-tungstenite with rustls for the Price source feed, and a pinned toolchain with strict lints.
Agents write the code, and the engineer does not read Rust fluently, so quality comes from guardrails rather than line-by-line review.
Rust gives more of those guardrails for free than TypeScript.
The compiler rejects whole classes of bugs, and a crate that does not depend on an adapter cannot import it, so the ports-and-adapters dependency rules fail the build instead of a separate check.
The cost is two languages in one repo and slower builds in the agent loop.
The evidence behind each choice is in the [research on Rust project standards](../research/rust-project-standards.md).

## Stack

- **HTTP.** axum on tokio, with `tower-http` for static files, compression, timeouts and request tracing.
  axum has built-in SSE with keep-alive and about 11 times the downloads of actix-web.
- **Store.** sqlx with `query!` macros checked at compile time against committed offline `.sqlx` metadata, embedded migrations, and a write pool of exactly one connection.
  Every store adapter must pass one shared test suite for the store port.
  That suite makes swapping the store safe, not sqlx's Postgres support.
- **Price source feed.** `tokio-tungstenite` with rustls and `webpki-roots`.
  The binary installs exactly one rustls crypto provider at start-up, because without one the first `wss://` connect panics.
- **Errors.** `thiserror` in the domain and adapters, `anyhow` only in the binary.
  The HTTP layer maps each domain error to a status code explicitly.
- **Configuration.** `clap` with its `env` feature reads environment variables once at start-up.
  The server refuses to start on invalid configuration.
- **Logging.** `tracing` with `EnvFilter` from `RUST_LOG`, defaulting to `info`.
  `LOG_FORMAT` selects `text`, the default for local runs, or `json`.
  Terraform sets `json` on Render, so a later log stream can read the fields without parsing text.
- **Image.** `cargo-chef` for cached dependency layers, a glibc build, and `gcr.io/distroless/cc-debian13:nonroot` at runtime.

## Guardrails

All of these run in the Green gate, and any warning fails it.

- `rust-toolchain.toml` pins an exact stable version with `clippy` and `rustfmt`, the crates use edition 2024, and `rust-version` matches the pin.
  Update bots propose new versions.
- The workspace `[lints]` table forbids `unsafe_code`, denies `clippy::pedantic` at `priority = -1`, and denies `allow_attributes` and `allow_attributes_without_reason`, so every exception is an `#[expect(..., reason = "...")]`.
- It also denies `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`, `dbg_macro`, `print_stdout`, `print_stderr` and `indexing_slicing`.
  `clippy.toml` allows `unwrap`, `expect`, `panic` and `dbg!` in tests.
- Clippy runs with `-- -D warnings`, not through `RUSTFLAGS`, which would invalidate the build cache.
- The gate also runs `rustfmt --check` and `cargo sqlx prepare --workspace --check`.
- `cargo-deny` checks advisories, licences and duplicate crates, and bans `openssl`.
- `cargo-shear` reports unused dependencies and source files no module includes.
- `cargo-nextest` runs the tests and `cargo test --doc` the doctests, and tests use hand-written fakes for the ports instead of `mockall`.

## Considered Options

- **TypeScript on Node.** Rejected.
  It shares types directly with the React frontend and the engineer can read every diff, but the map in issue #1 had already decided that guardrails define quality, not review.
  `strict` mode and ESLint let `any`, unhandled promises and runtime type gaps through, and layer rules need a separately configured tool such as dependency-cruiser.
- **actix-web, poem or loco instead of axum.** Rejected.
  actix-web has its own runtime model and a smaller middleware ecosystem.
  loco imposes a Rails-style structure that conflicts with ports and adapters.
- **rusqlite instead of sqlx.** Rejected, narrowly.
  It fits the single writer more directly, but its SQL is only checked by tests, while sqlx checks it at compile time and, from 0.9, flags queries built with `format!`.
- **A static musl image.** Rejected.
  It complicates cross-compiling the TLS library and SQLite, and musl's allocator is slower.

## Consequences

- The acceptance test suite talks to the server over HTTP and the UI, so it may use another language.
  ADR 0011 chooses TypeScript with Playwright.
- The frontend shares no code with the server.
  Its types are generated from the server's wire types with `ts-rs` (ADR 0014).
- axum 0.8 has unreleased breaking changes queued, so a 0.9 release during the demo means a small migration.
- sqlx 0.9 requires Rust 1.94, and `sqlx-cli` is pinned with `--version`, because `cargo install --locked` no longer works for it.
- Render does not document whether a `nonroot` container may write to its persistent disk.
  A probe on 2026-10-31 showed it can: Render mounts the disk as `root:1000` with mode `2775` and adds the process to group 1000, so the image stays `nonroot` ("Check the price feeds from Render Frankfurt").
