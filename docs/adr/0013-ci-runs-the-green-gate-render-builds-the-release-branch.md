# CI runs only the Green gate, and Render builds and deploys a `release` branch

GitHub Actions runs the Green gate and nothing else, on every push to `main` and every pull request against `main`.
A Release fast-forwards a `release` branch to a `main` commit whose Green gate passed, and Render builds the Dockerfile from that branch and deploys it.
This keeps the pipeline free of image publishing and secrets, and every deploy is a deliberate step rather than a side effect of a commit.

## Decision

### Pipeline

- **Triggers.** `push` to `main` and `pull_request` against `main`, nothing else.
- **Superseded runs.** One concurrency group per workflow and ref with `cancel-in-progress`, so a newer run cancels the older one on `main` and on a pull request alike.
- **One job, one command.** The job runs `mise run gate`, the same command agents and `docs/agents/verification.md` call (ADR 0012), with caches for Cargo, pnpm and the Playwright browsers.
- **Task runner.** `mise` defines the Green gate and the other tasks, and pins Node, pnpm, `cargo-nextest`, `cargo-deny` and `cargo-shear`; the Rust toolchain stays in `rust-toolchain.toml` (ADR 0006).
- **Actions.** The workflows pin third-party actions by commit SHA, and Dependabot updates them weekly; it updates nothing else.
- **No ruleset.** There is no branch protection and no required check, because work lands trunk-based on `main`.

### Release

- **`mise run release [sha]`**, run by hand, makes a Release.
  It takes `origin/main` unless a SHA is given, checks with `gh` that the pipeline run on `main` for that SHA succeeded, and stops without pushing if it is running, failed or missing.
  Then it runs `git push origin <sha>:release`, which git refuses unless it is a fast-forward.
- **Render** is connected to `release` with auto-deploy "On Commit", builds the Dockerfile itself (ADR 0006) and deploys it.
  Terraform, applied by hand, describes the branch and the auto-deploy setting.
- **Rollback** uses Render's rollback in the dashboard; the task never force-pushes `release`.

### Checks against the deployed service

Both run by hand from the acceptance package, outside the Green gate and without control channels (ADR 0011):

- **`pnpm smoke`** after every Release: the page loads, a price is visible, and the SSE stream delivers its snapshot.
  The health endpoint reports Render's `RENDER_GIT_COMMIT`, so the smoke test also checks that the released SHA is live.
- **`pnpm smoke:hold`** once after the first Release holds an SSE stream open for an hour to show that Render's proxy does not cut it.
  If it does, the heartbeat interval changes, not the architecture.

## Considered Options

- **GitHub Actions builds the image, pushes it to GHCR, and calls Render's deploy hook.** Rejected for now.
  It runs exactly the image the pipeline built, spends no Render build minutes, and could run the smoke test in the same pipeline, but it needs an image job, a public registry package and a deploy-hook secret.
- **Render's "After CI Checks Pass" on `main`.** Rejected.
  Every green commit would deploy and spend Render build minutes, and a smoke test in the same pipeline would deadlock.
  Render waits for every check of the commit, and the smoke test waits for Render.
- **"After CI Checks Pass" on `release`.** Rejected.
  After a fast-forward, the commit's checks come from its run on `main`, and Render's docs do not say whether it counts them; the release task checks the gate instead.
- **A merge commit as a Release.** Rejected, because it creates a SHA without a gate run and lets `release` drift from `main`.
- **Parallel jobs for the parts of the gate.** Rejected for now, because CI would no longer run the one command everyone else runs.
- **`just` as the task runner.** Rejected, because tool versions would then be pinned in a second place.
- **The one-hour SSE hold in every pipeline or on a schedule.** Rejected.
  It tests a fact about the platform, not about a commit, and a cut stream costs a client only a reconnect with a fresh snapshot (ADR 0010).

## Consequences

- Render builds the image that runs, not the pipeline.
  The Green gate tests the local build, not the image, so a broken Dockerfile shows up only on a Release.
- Every Release spends Render build minutes; a Hobby workspace includes 500 Starter-tier minutes a month.
- You apply Terraform changes by hand against HCP Terraform Free, so the pipeline holds no Render or HCP credentials.
- What is live is whatever `release` points to, so `git log release` is the deploy history.
