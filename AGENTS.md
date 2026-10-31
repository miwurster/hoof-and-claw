## Coding standards

Read `CODING_STANDARDS.md` before you write code that handles a price, the time, a state, a store port, an error, or an acceptance case.

## Architecture

Read `ARCHITECTURE.md` before you add a crate, package, module, port or adapter, wire the server, or add a component or test layer.

## Working rules

- Run `mise run gate` before you hand work over; it is the Green gate.
- Prove a finished change with the `verification` skill.
- The ADRs in `docs/adr/` hold the reasons; `ARCHITECTURE.md`, `CODING_STANDARDS.md` and `docs/agents/verification.md` hold the rules.
  An ADR change that changes a rule updates the file holding that rule in the same commit.

## Agent skills

### Issue tracker

Issues and specs live in GitHub Issues of `miwurster/hoof-and-claw`, managed with the `gh` CLI.
See `docs/agents/issue-tracker.md`.

### Triage labels

The five default triage labels: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`.
See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: one `GLOSSARY.md` and `docs/adr/` at the repo root.
See `docs/agents/domain.md`.
