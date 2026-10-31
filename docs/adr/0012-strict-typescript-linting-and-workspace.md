# Strict TypeScript linting with Oxlint and anti-slop, in a pnpm workspace without Nx

All TypeScript in the repo, the React SPA (ADR 0005) and the acceptance suite (ADR 0011), follows one strict set of compiler options and lint rules.
Agents write the code, so these rules are part of the guardrails that replace line-by-line review (ADR 0006).
The TypeScript packages live in a pnpm workspace next to the Cargo workspace, and one task runner runs the whole Green gate as one command; there is no Nx.

## Decision

### Compiler

Every package's `tsconfig` turns on `strict`, `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`, `noImplicitOverride` and `verbatimModuleSyntax`.

### Lint

Oxlint lints every TypeScript package:

- **Categories.** `correctness`, `suspicious` and `pedantic` are errors.
- **Type-aware rules.** These include `no-floating-promises`, which catches a Playwright step or a `fetch` that nobody awaits.
- **Escapes.** `any`, the non-null assertion `!`, and `@ts-ignore` are forbidden.
- **anti-slop.** The repo holds a copy of the rules of `dmmulroy/anti-slop` in `tools/oxlint/anti-slop/`, as its README intends, with Oxlint and `@oxlint/plugins` pinned to the exact versions it needs.
  All its rules except the Effect rules are errors.
  Among other things, they forbid `unknown` in parameters, returns and type aliases, chained type assertions, a type assertion without a `// SAFETY:` comment, ad hoc `typeof` checks, and module mocking.
- **React.** In the SPA, Oxlint's `react`, `react-hooks` and `jsx-a11y` plugins are errors.
  The Stylelint rule on raw colours from ADR 0005 stays.
- **Playwright.** In `acceptance/`, `eslint-plugin-playwright` runs through Oxlint's JS plugins, with rules such as `no-wait-for-timeout`, `no-force-option` and `prefer-web-first-assertions`.
  If a rule does not run there, ESLint runs beside Oxlint with only that plugin.
- **Zero warnings.** A warning fails the gate like an error.

A formatter (Prettier or oxfmt) formats every package.
dependency-cruiser enforces the layers of the acceptance suite (ADR 0011) and the container and presentational split of the SPA (ADR 0005).

### Workspace

- A **pnpm workspace** holds the TypeScript packages: the SPA and `acceptance/`, and later a package for shared API types if the API contract calls for one.
- The **Cargo workspace** holds the server's crates (ADR 0008).
- **One task runner**, `mise`, defines the Green gate as one command (ADR 0013).
  Agents, CI and `docs/agents/verification.md` all call that command.

## Considered Options

- **ESLint with `typescript-eslint`'s strict type-checked configs.** Rejected.
  It is the most complete rule set, but anti-slop is built for Oxlint, and Oxlint is fast enough to lint on every change.
- **Biome.** Rejected, because it cannot run anti-slop or the Playwright rules.
- **Nx as the monorepo tool.** Rejected for now.
  Its caching and `affected` runs pay off with many packages, but the repo has two or three TypeScript packages and a Cargo workspace.
  Nx supports Rust only through a community plugin, while Cargo already caches its own builds.
  Its boundary rule is an ESLint rule, while dependency-cruiser and the crate-graph test already enforce the boundaries.
  Its configuration and plugin versions would be one more thing agents have to understand and maintain.

## Consequences

- Because anti-slop lives in the repo, an update means copying a new version and pinning Oxlint to match.
- Where a rule forces an awkward construct, the code disables it for that line with a reason, never for a whole file.
- Nx comes back into question when the repo has many more packages, or when the Green gate's time in CI becomes the bottleneck.
