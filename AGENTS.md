## Coding standards

This repo's own code rules, on top of the Kipu baseline the review lenses carry, are in `CODING_STANDARDS.md`.
Read it before changing code: its rules are normative and win over the baseline on the same concern.

## Architecture

Read `ARCHITECTURE.md` before you add a crate, package, module, port or adapter, wire the server, or add a component or test layer.

## Working rules

- Run `mise run gate` before you hand work over; it is the Green gate.
- Write unit and domain tests first.
- Write each acceptance criterion as a test case in the acceptance DSL before the code that makes it pass.
- Prove a finished change with the `verification` skill; how this repo is verified is in `docs/agents/verification.md`.
- The ADRs in `docs/adr/` hold the reasons; `ARCHITECTURE.md`, `CODING_STANDARDS.md` and `docs/agents/verification.md` hold the rules.
  A change to an ADR that changes a rule updates the file holding that rule in the same commit.

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
