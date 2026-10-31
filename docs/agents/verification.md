# Verification

How the `verification` skill proves a change in this repo.
Read the routes from the code, not from this file: the game's routes from the router in `crates/web`, the test controls from `crates/test-controls`.

## Green gate

`mise run gate`, the same command CI and every agent run.

## Launch

`PORT=<port> DATABASE_PATH=<file> mise run launch` builds the SPA and `server` with the Cargo feature `test-controls` in the Cargo profile `gate` (ADR 0015) and starts the server on an empty SQLite file, migrations included.
It sets `RENDER_GIT_COMMIT` to `git rev-parse HEAD`, so the instance reports the commit it was built from.

- **Ready signal:** the server logs `listening on <address>` once migrations ran; `GET /health` then answers 200.
- **Two instances side by side:** give each its own `PORT` and `DATABASE_PATH`; `PORT=0` picks a free port, read from the log line.
  The acceptance suite's worker fixture starts its servers the same way.
- **Teardown:** stop the process by its PID, then delete the database file.
- **Start state:** the price is Price delayed and there is no price until a test control sends one.

## Doctor

`curl -s localhost:<port>/health` reports the commit as `build`, which must equal `git rev-parse HEAD` of the launched tree, and `"testControls": true`.

## Drive

- **HTTP API:** `curl` with a cookie jar (`-c jar -b jar`).
  The first request issues the identity cookie; every later request of the same Player sends the same jar, and a new Player is a new jar.
- **SSE stream:** `curl -N -b jar` on the stream route; the first event is the snapshot.
- **Web UI:** a Playwright browser session against `http://localhost:<port>/`, finding elements by role and accessible name.

## Acceptance suite

`acceptance/`, Playwright Test, cases written in the TypeScript DSL in `acceptance/cases/`.

- Run all: `pnpm --filter acceptance test`.
- Run the performance cases and their budgets: `mise run perf`; each case writes its numbers to `measurements.json` in its folder under `acceptance/test-results/performance/`, which CI keeps as the artefact `performance`.
- Run one case alone: `pnpm --filter acceptance test --project api -g "<case title>"`, and again with `--project ui`.
- A new case is written in the DSL, for example:

  ```ts
  await priceSource.sends("64,000.00");
  await alice.guesses("Up");
  await clock.advance(seconds(60));
  await priceSource.sends("64,000.01");
  await alice.should.seeOutcome("Correct");
  await alice.should.haveScore(1);
  ```

## Observe

Side effects: none beyond the public interface.
Score, Outcome and the Open guess are read through the HTTP API and the SSE stream, never from the SQLite file.

Time and price: only the four `test-controls` commands, at the routes `crates/test-controls` defines.

1. Send a price.
2. Lose the connection to the Price source, or restore it.
3. Advance the clock.
4. Set the clock to a time.

The clock stands still until a command moves it, and a guess resolves only on a received price.
