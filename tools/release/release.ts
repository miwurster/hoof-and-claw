// `mise run release [sha]`: fast-forwards `release` to a `main` commit whose Green gate passed (ADR 0013).
// Render builds and deploys `release` on every commit; a rollback happens in Render's dashboard, never here.
import { execFileSync } from "node:child_process";

/** A run of the Green gate workflow, as `gh run list --json status,conclusion` reports it. */
type Run = { readonly status: string; readonly conclusion: string };

function refuse(reason: string): never {
  process.stderr.write(`Not released: ${reason}\n`);
  process.exit(1);
}

/** Runs a command and returns its output, or refuses the Release with `failure` when the command fails. */
function run(failure: string, command: string, ...args: string[]): string {
  try {
    return execFileSync(command, args, { encoding: "utf8", stdio: ["ignore", "pipe", "inherit"] }).trim();
  } catch {
    return refuse(failure);
  }
}

const target = process.argv[2] ?? "origin/main";

run("git could not fetch origin.", "git", "fetch", "--quiet", "origin");

const sha = run(`${target} is no commit.`, "git", "rev-parse", "--verify", `${target}^{commit}`);

const output = run(
  `gh could not list the Green gate runs for ${sha}.`,
  "gh",
  "run",
  "list",
  "--workflow",
  "gate.yml",
  "--branch",
  "main",
  "--event",
  "push",
  "--commit",
  sha,
  "--json",
  "status,conclusion",
);

// SAFETY: `gh run list --json status,conclusion` prints an array of objects with exactly these two string fields.
// oxlint-disable-next-line typescript/no-unsafe-type-assertion -- JSON.parse returns any.
const runs = JSON.parse(output) as readonly Run[];

// `gh` lists the newest run first, and a re-run replaces its run's status and conclusion.
const latest = runs[0];

if (latest === undefined) {
  refuse(`there is no Green gate run on main for ${sha}.`);
}

if (latest.status !== "completed") {
  refuse(`the Green gate run on main for ${sha} is still running (${latest.status}).`);
}

if (latest.conclusion !== "success") {
  refuse(`the Green gate run on main for ${sha} ended in ${latest.conclusion}.`);
}

// Without --force, git refuses anything but a fast-forward.
run(`git refused to fast-forward release to ${sha}.`, "git", "push", "--quiet", "origin", `${sha}:refs/heads/release`);

process.stdout.write(`Released ${sha}: Render builds and deploys release.\n`);
