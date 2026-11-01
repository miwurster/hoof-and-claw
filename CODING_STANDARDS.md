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
- The HTTP layer maps each domain error to a status code and an error code explicitly, never through a catch-all.
- Every error under `/api/` and `/test-controls/` is `{ "error": "<code>" }` with a code from the generated error enum, the axum extractors' rejections and the rate limiter's answer included; no route answers with plain text or HTML.

## API contract

ADR 0014 fixes the routes, payloads and status codes.

- A wire type is never a domain type: `web` converts between them, so a change in a core does not change the contract by accident.
- JSON field names are camelCase; prices on the wire are integer cents; enum values are lower camelCase strings (`"up"`, `"correct"`).
- An SSE event is one JSON object in `data:` with a `type` field, with no `event:`, `id:` or `retry:` field.
- An event carries absolute values, never deltas, so applying it twice changes nothing.
- The client decodes a response body or an event in exactly one place, the game hook, with one `as` to the generated type.
- Only the game hook reconnects the stream; it closes the old `EventSource` first and waits a random, growing delay.

## Lint exceptions

- A Rust exception is `#[expect(lint, reason = "...")]` on the narrowest item, never `#[allow]`.
- A TypeScript exception disables one rule for one line with a reason, never for a file.
- A TypeScript type assertion (`as`) carries a `// SAFETY:` comment saying why it holds.

## Acceptance tests

- A test case speaks only in glossary terms through the DSL: no endpoint, selector, cookie or status code appears in `cases/`.
- A price in a test case is a dollar string such as `"64,000.00"`; the drivers convert it to cents.
- The UI driver finds an element by its role and accessible name, and uses `data-testid` only where no role fits.
- A test waits for an expected state with a timeout (`expect`, `expect.poll`), never for a fixed time.
  The one exception is the performance cases, which wait a little more than the price throttle's second before each measured price, so they measure only the first change after a quiet second (ADR 0010, ADR 0015).
- A DSL assertion's failure message uses glossary terms, such as "Alice should have Score 1, but has 0".
