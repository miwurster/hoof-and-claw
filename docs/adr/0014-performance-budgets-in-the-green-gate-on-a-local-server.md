# Performance budgets in the Green gate, on a local server only

The performance test layer guards against regressions.
It drives 200 Players against a local server, measures five metrics, and fails the Green gate when one of them exceeds its budget.
Load never goes to the deployed service on Render, so no load reaches Bitstamp, Render's included traffic, or the per-IP limits of the deployed service.
It does not prove capacity on Render Starter, and it does not look for a breaking point; the scale path in ADR 0003 stays an estimate.

## Decision

### Where it runs

- **Only against a local server**, on the developer's machine or the CI runner, started with `mise run launch` like the acceptance suite's servers (ADR 0011).
  The controllable `PriceSource` and `Clock` from `test-controls` drive price and time, so the run never touches Bitstamp.
- **Never against the deployed service.**
  The checks against Render stay `pnpm smoke` and `pnpm smoke:hold` (ADR 0013).
- **Rate limits raised for the run.**
  All load comes from `127.0.0.1`, so the run sets the per-IP limits and the cap on concurrent SSE streams per IP high through configuration.
  The acceptance suite checks that the limits hold.

### Build

`mise run launch` builds `server` with `--features test-controls` in a Cargo profile `gate`, which inherits from `release` but turns off LTO and uses `codegen-units = 16`.
The acceptance suite, the performance run and `docs/agents/verification.md` share this one build, so the Green gate compiles the server once.
A debug build would say nothing about performance, and the full `release` profile compiles too slowly for every gate.
The Dockerfile Render builds keeps the `release` profile.

### Where the cases live

The performance cases live in `acceptance/performance/`, next to `cases/`, and like `cases/` they import only from `dsl/`; dependency-cruiser enforces it.
They run in their own Playwright project, `performance`, with one worker and only the API driver.
That project is not one of the default projects.
The DSL gains group actors such as `players(200)` and steps that measure.
Percentiles come from `hdr-histogram-js`.

### Load profile and budgets

200 concurrent Players, about ten times the tens of Players the demo expects, each holding one SSE stream.
The frozen clock is advanced rather than waited for, so no case waits 60 real seconds.

| Metric | Budget |
| --- | --- |
| Price fan-out: from sending a price to its arrival on the last of 200 streams | p95 < 250 ms, max < 1 s |
| Placing a Guess, the HTTP request | p95 < 100 ms |
| Resolution burst: 200 Guesses due on one price, until the last Guess resolved event | < 2 s |
| Snapshot: from opening a stream to its first event | p95 < 200 ms |
| Resident memory of the server with 200 streams open | < 128 MB |
| Errors and streams closed by the server | 0 |

- **Ten times headroom.**
  Shared CI runners vary from run to run, and retries are 0 (ADR 0011).
  So each budget is about ten times the expected value.
  That still catches a query per stream or a fan-out that grows with the square of the streams, but not noise.
- **Throttle.** The price throttle runs on real time and lets the first change after a quiet second out at once (ADR 0010).
  The harness waits more than one second between two prices, so every fan-out it measures is such a first change.
- **Budgets are configuration** of the harness, not constants in a case.
  If the first real run shows them far off, we adjust them once; after that, a change needs a reason in the commit.
- **Measurements as artefacts.** Every run writes its numbers to a file that CI keeps as an artefact.

### In the Green gate

`mise run gate` runs the performance cases after the acceptance suite, never in parallel with it, so Playwright's workers do not compete for the same CPUs.
The run takes at most about 60 s.
`mise run perf` runs it alone.

## Considered Options

- **k6.** Rejected.
  It is the common choice and has thresholds built in, but it has no native SSE.
  SSE needs the `xk6-sse` extension and a custom k6 binary that `mise` cannot simply pin, and the scripts would be a third world beside the Rust server and the TypeScript suite.
- **Goose, in Rust.** Rejected.
  It has no SSE either and would be a second driver layer beside the acceptance suite's.
- **A separate `performance/` workspace package.** Rejected.
  `acceptance/` would have to export its drivers, or the drivers would move into a third package, while a folder beside `cases/` reuses the DSL, the worker fixture and the lint rules as they are.
- **Comparing against a stored baseline**, such as "no more than 20 % slower than last week". Rejected.
  It needs stable hardware, and on shared runners it would turn the gate red on noise.
- **Load against the deployed service.** Rejected for now.
  It would meet the per-IP limits, spend Render's included traffic, and slow the game down for real Players during the run.
- **A separate debug build for the acceptance suite and a release build for performance.** Rejected, because the gate would compile the server twice.

## Consequences

- The Green gate gets slower by the performance run and by compiling with optimisations; the `gate` profile keeps the second cost down.
- Nothing measures the service on Render Starter's 0.5 CPU and 512 MB.
  The memory budget guards against regressions; it is not a capacity proof.
- A change that makes the server slower but stays inside the budgets passes.
  The artefacts show the trend for anyone who looks.
