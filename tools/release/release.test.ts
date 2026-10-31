// The release task against a stubbed `gh` and a local `origin` (ADR 0013).
import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import { chmodSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, test } from "node:test";

const RELEASE = join(import.meta.dirname, "release.ts");

// Answers `gh run list` with the runs in GH_STUB_RUNS, but only when asked about GH_STUB_SHA.
const GH_STUB = `#!/bin/sh
case " $* " in
  *" --commit $GH_STUB_SHA "*) printf '%s\\n' "$GH_STUB_RUNS" ;;
  *) echo '[]' ;;
esac
`;

type Run = { readonly status: string; readonly conclusion: string };

type Outcome = { readonly code: number | null; readonly output: string };

let dir = "";
let clone = "";
let stubs = "";

function git(cwd: string, ...args: string[]): string {
  return execFileSync("git", args, { cwd, encoding: "utf8" }).trim();
}

function commit(message: string): string {
  git(clone, "commit", "--allow-empty", "--quiet", "-m", message);

  return git(clone, "rev-parse", "HEAD");
}

function pushMain(): void {
  git(clone, "push", "--quiet", "origin", "HEAD:main");
}

/** The commit `release` points to on `origin`, or `undefined` before the first Release. */
function released(): string | undefined {
  const line = git(clone, "ls-remote", "origin", "refs/heads/release");

  return line === "" ? undefined : line.split("\t")[0];
}

/** Runs the release task in the clone, with `gh` reporting `runs` for the commit `sha`. */
function release(sha: string, runs: readonly Run[], ...args: string[]): Outcome {
  const result = spawnSync("node", [RELEASE, ...args], {
    cwd: clone,
    encoding: "utf8",
    env: {
      ...process.env,
      PATH: `${stubs}:${process.env["PATH"] ?? ""}`,
      GH_STUB_SHA: sha,
      GH_STUB_RUNS: JSON.stringify(runs),
    },
  });

  return { code: result.status, output: `${result.stdout}${result.stderr}` };
}

beforeEach(() => {
  dir = mkdtempSync(join(tmpdir(), "hoof-and-claw-release-"));
  clone = join(dir, "clone");
  stubs = join(dir, "bin");

  execFileSync("git", ["init", "--quiet", "--bare", "--initial-branch", "main", join(dir, "origin.git")]);
  execFileSync("git", ["clone", "--quiet", join(dir, "origin.git"), clone], { stdio: "ignore" });
  git(clone, "config", "user.name", "Operator");
  git(clone, "config", "user.email", "operator@example.com");

  execFileSync("mkdir", [stubs]);
  writeFileSync(join(stubs, "gh"), GH_STUB);
  chmodSync(join(stubs, "gh"), 0o755);
});

afterEach(() => {
  rmSync(dir, { recursive: true, force: true });
});

await test("refuses to push when the main pipeline run for the commit is missing", () => {
  const sha = commit("first");

  pushMain();

  const outcome = release(sha, [], sha);

  assert.notEqual(outcome.code, 0);
  assert.match(outcome.output, /no Green gate run on main/u);
  assert.equal(released(), undefined);
});

await test("refuses to push while the main pipeline run for the commit is still running", () => {
  const sha = commit("first");

  pushMain();

  const outcome = release(sha, [{ status: "in_progress", conclusion: "" }], sha);

  assert.notEqual(outcome.code, 0);
  assert.match(outcome.output, /still running/u);
  assert.equal(released(), undefined);
});

await test("refuses to push when the main pipeline run for the commit failed", () => {
  const sha = commit("first");

  pushMain();

  const outcome = release(sha, [{ status: "completed", conclusion: "failure" }], sha);

  assert.notEqual(outcome.code, 0);
  assert.match(outcome.output, /failure/u);
  assert.equal(released(), undefined);
});

await test("fast-forwards release to the commit when its main pipeline run succeeded", () => {
  const sha = commit("first");

  pushMain();

  const outcome = release(sha, [{ status: "completed", conclusion: "success" }], sha);

  assert.equal(outcome.code, 0, outcome.output);
  assert.equal(released(), sha);
});

await test("without a commit, releases the newest commit on main at origin", () => {
  commit("first");
  pushMain();

  // Another clone moves main on origin after this clone last fetched.
  const other = join(dir, "other");

  execFileSync("git", ["clone", "--quiet", join(dir, "origin.git"), other], { stdio: "ignore" });
  git(
    other,
    "-c",
    "user.name=Agent",
    "-c",
    "user.email=agent@example.com",
    "commit",
    "--allow-empty",
    "--quiet",
    "-m",
    "second",
  );
  git(other, "push", "--quiet", "origin", "HEAD:main");

  const newest = git(other, "rev-parse", "HEAD");
  const outcome = release(newest, [{ status: "completed", conclusion: "success" }]);

  assert.equal(outcome.code, 0, outcome.output);
  assert.equal(released(), newest);
});

await test("refuses a commit that is not ahead of the released one, because release only fast-forwards", () => {
  const first = commit("first");
  const second = commit("second");

  pushMain();
  assert.equal(release(second, [{ status: "completed", conclusion: "success" }], second).code, 0);

  const outcome = release(first, [{ status: "completed", conclusion: "success" }], first);

  assert.notEqual(outcome.code, 0);
  assert.equal(released(), second);
});
