# Acceptance suite in TypeScript with Playwright, one server per worker

The acceptance test suite is a TypeScript package, `acceptance/`, run by Playwright Test.
It follows Dave Farley's four layers: test cases in domain language, a domain-specific language (DSL), protocol drivers, and the system under test.
Every test case runs through two drivers, the UI and the HTTP API, against a local `server` built with the Cargo feature `test-controls` (ADR 0008).
Each Playwright worker gets its own server process, so the suite runs in parallel while every test has the server's clock and price to itself.
This settles what ADR 0006 left open about the suite's language and what ADR 0008 left open about how the suite uses `test-controls`.

## Decision

### Layers

`acceptance/` has one folder per layer:

- `cases/` holds the test cases.
  They import only from `dsl/`.
- `dsl/` holds the DSL.
  It talks only to a driver interface.
- `drivers/ui/` drives the SPA in a browser.
- `drivers/api/` drives the HTTP API and the SSE stream.
- `drivers/controls/` calls the `test-controls` endpoints.

No driver imports another.
dependency-cruiser checks these rules in the Green gate, as the crate-graph test does for the server (ADR 0008).

### The DSL

Test cases read in glossary terms and never name endpoints, selectors or cookies:

```ts
await priceSource.sends("64,000.00");
await alice.guesses("Up");
await clock.advance(seconds(60));
await priceSource.sends("64,000.01");
await alice.should.seeOutcome("Correct");
await alice.should.haveScore(1);
```

- **Actors.** Players are named by alias (`alice`, `bob`), and each alias is a fresh Player.
  `priceSource` and `clock` control the system.
- **Fluent assertions.** Assertions belong to the DSL as much as actions do.
  An actor offers them under `should`.
  Custom Playwright matchers such as `expect(alice).toHaveScore(1)` are the fallback where a `should` step does not read naturally.
  Failure messages use glossary terms, such as "Alice should have Score 1, but has 0".
- **Prices** are written as dollar strings, and the drivers convert them to cents (ADR 0009).
- **Coming back.** `alice.closesTheBrowserAndReturns()` keeps the cookie and reopens the page, or the SSE stream in the API driver.
- **Restarting.** `server.restarts()` stops the worker's process, starts it again on the same SQLite file, and sets the clock back to the time the suite last set.
  It proves that an Open guess survives a deploy (ADR 0003).

### Two drivers for every case

Two Playwright projects, `ui` and `api`, choose the driver.
Every test case runs in both by default.
A case that only makes sense in one channel carries a tag for that channel, with the reason next to the tag.
An example is "Up and Down are disabled while the price is delayed", which only the UI can show.

- **The UI driver** finds elements by role and accessible name, the way a person reads the page, so the suite also checks accessibility.
  It uses `data-testid` only where no role fits.
- **The API driver** opens the Player's SSE stream as the client does and asserts on its snapshot and events (ADR 0010).
- **Waiting.** Both drivers wait for an expected state with a timeout (`expect`, `expect.poll`).
  No test sleeps, and a lint rule forbids `waitForTimeout` (ADR 0012).

### Time and price

`test-controls` offers four commands over HTTP:

1. **Send a price.** The controllable `PriceSource` delivers it as a received price.
2. **Lose the connection, or restore it.** This switches between Price delayed and live.
   The source starts in Price delayed, as the real server does (ADR 0007).
   So a test that wants to play first sends a price, which the DSL does in one step.
3. **Advance the clock.**
4. **Set the clock to a time.** `server.restarts()` needs it.

The controllable `Clock` stands still until a test moves it.
Resolution happens on a received price (ADR 0010), so a test places a guess, advances the clock, sends a price and checks the Outcome, with no race.
In the UI driver, `clock.advance` also moves the browser's clock with `page.clock`, so the countdown follows the server.

The `Clock` port is the domain's time only: when a guess was placed and when it falls due.
The price throttle and the SSE keepalive from ADR 0010 run on `tokio`'s real time, because they are infrastructure.
A second price within the same second therefore reaches the test after at most one real second, and the waiting assertions absorb that.

### Isolation

- **One server per worker.** A global setup builds the SPA and `server` with `--features test-controls` once.
  A worker fixture starts one process per worker on port 0 with its own temporary SQLite file and waits for it to be ready.
- **Parallel.** Playwright runs one worker per CPU core by default, and the tests inside a worker run one after another.
  So a test has its server's clock and price to itself.
- **Fresh state per test.** Before each test, the fixture resets the `PriceSource` to its start state: Price delayed and no price.
  The clock only moves forward and is not reset.
  Each test uses fresh Players through a new browser context or cookie, so Open guesses left by earlier tests belong to other Players and do no harm.

### Where it runs

The suite runs in the Green gate, locally and in CI, against the local build with `test-controls`.
It never runs against the deployed service, which has no control channels and a real price.
The checks against the deployed service, `pnpm smoke` after every Release and `pnpm smoke:hold`, which holds an SSE stream open for an hour (ADR 0003), are a separate small set without control channels, run by hand (ADR 0013).

- **Browser.** Only Chromium runs in the Green gate.
- **No retries.** Retries are 0, so a flaky test turns the gate red instead of slipping through.
- **Artefacts.** On a failure Playwright keeps the trace, a screenshot and the worker's server log.

### Acceptance test-driven

Every acceptance criterion of the spec is written as a test case in the DSL before the code that makes it pass, and the spec points to its cases.
The rules of the brief form the first set: the visible Score and price, one guess at a time, resolution after at least 60 seconds and a changed price, plus or minus one, and a start at 0.

## Considered Options

- **The suite in Rust, in an `acceptance` crate as ADR 0008 first said.** Rejected.
  Rust's browser drivers (`fantoccini`, `chromiumoxide`) are thin, with no auto-waiting, traces or control over the browser's clock.
  A TypeScript suite shares language and tooling with the SPA.
- **Drivers in one language and cases in another.** Rejected, because it is two toolchains for one suite.
- **One server per test.** Rejected.
  It isolates best, but every test pays a start-up, while a worker already runs its tests one at a time.
- **One shared server with a clock and price per test, chosen by a header.** Rejected.
  It makes the server under test differ from the real one and adds routing for test state.
- **A clock that runs in real time plus an offset.** Rejected.
  Tests would race the 60-second boundary.
- **The price throttle on the `Clock` port.** Rejected.
  With a frozen clock a second price within one second would never go out, and making `priceSource.sends` advance the clock would shift every 60-second boundary a test computes.
- **Retries in the gate.** Rejected, because they hide flaky tests.

## Consequences

- ADR 0008's `acceptance` crate becomes the TypeScript package `acceptance/`, and `test-controls` gains the command to set the clock.
- The API driver depends on the API contract between frontend and server, which is not yet specified.
- The gate tests only Chromium, so it does not catch bugs that only Firefox or Safari show.
- `docs/agents/verification.md` drives the app through the same drivers and control channels.
