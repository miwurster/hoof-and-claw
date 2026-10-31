# Coding standards

This repo's own code rules, on top of the code review baseline.
A rule here wins over a lens default on the same concern.
Where a rule needs a concrete crate, package or module, `ARCHITECTURE.md` owns it.
What the compiler, Clippy, Oxlint, anti-slop, Stylelint or cargo-deny already enforce is configured there and not repeated here.

## Prices

- In the server, a price is a newtype over an unsigned integer number of cents, never a float and never a decimal type.
- An adapter parses the source's exact decimal string (`price_str`, the ticker's `last`) straight into cents.
  A price with more than two decimal places is an error, never rounded.
- A price shown to a Player or used as a Start price or End price is a trade price; never the mid price, bid, ask or another ticker value.

## Time

- The domain's time comes only from the `Clock` port: when a guess was placed and when it falls due.
  A core takes the time as a value and never reads a clock.
- Infrastructure timing, such as the price throttle, the SSE keepalive, heartbeats and reconnect backoff, runs on `tokio`'s real time, never on `Clock`.
- The client never decides an Outcome and never compares a server time with its own clock.
  It counts down from when it received the server's remaining time, and only the server's resolution event ends an Open guess.

## State and persistence

- A state is an enum whose variants carry only the fields that state has; a Guess is `Open` or `Resolved`, never a status string with optional fields.
- Identifiers are newtypes, and a constructor checks the invariants, so an invalid value cannot be built.
- A store port offers operations in domain language, such as placing a Guess or resolving one together with its Score change in one call; no generic create, read, update or delete.
- The server persists a Guess before it acknowledges it, and persists a resolution before it publishes the resolution events.
- Resolving a Guess is a conditional update that only changes a Guess that is still open, so a retry cannot resolve it twice.

## Errors

- The cores and adapters return `Result` with an enum error type built with `thiserror`.
- The HTTP layer maps each domain error to a status code explicitly, never through a catch-all.

## Lint exceptions

- A Rust exception is `#[expect(lint, reason = "...")]` on the narrowest item, never `#[allow]`.
- A TypeScript exception disables one rule for one line with a reason, never for a file.
- A type assertion carries a `// SAFETY:` comment saying why it holds.

## Acceptance tests

- A test case speaks only in glossary terms through the DSL: no endpoint, selector, cookie or status code appears in `cases/`.
- A price in a test case is a dollar string such as `"64,000.00"`; the drivers convert it to cents.
- The UI driver finds an element by its role and accessible name, and uses `data-testid` only where no role fits.
- A test waits for an expected state with a timeout (`expect`, `expect.poll`), never for a fixed time.
- A DSL assertion's failure message uses glossary terms, such as "Alice should have Score 1, but has 0".
