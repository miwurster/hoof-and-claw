// One server process per Playwright worker, started with `mise run launch` on port 0 and its own SQLite file (ADR 0011).
import { execFileSync, spawn } from "node:child_process";
import type { ChildProcess } from "node:child_process";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createInterface } from "node:readline";

const READY = /listening on \S+:(\d+)/u;
const START_TIMEOUT_MS = 180_000;
const STOP_TIMEOUT_MS = 20_000;

export type ServerProcess = {
  readonly baseURL: string;
  readonly build: string;
  readonly log: () => string;
  readonly stop: () => Promise<void>;
};

/** The commit `mise run launch` reports as the server's build. */
export function launchedCommit(): string {
  return execFileSync("git", ["rev-parse", "HEAD"], { encoding: "utf8" }).trim();
}

/** Starts a server on an empty database and resolves once it logs that it is listening. */
export async function startServer(): Promise<ServerProcess> {
  const dir = mkdtempSync(join(tmpdir(), "hoof-and-claw-"));
  const lines: string[] = [];

  const child = spawn("mise", ["run", "--quiet", "launch"], {
    cwd: join(import.meta.dirname, "..", ".."),
    detached: true,
    env: { ...process.env, PORT: "0", DATABASE_PATH: join(dir, "game.db"), NO_COLOR: "1" },
    stdio: ["ignore", "pipe", "pipe"],
  });

  createInterface({ input: child.stderr }).on("line", (line) => {
    lines.push(line);
  });

  const port = await new Promise<string>((resolve, reject) => {
    const timer = setTimeout(() => {
      reject(new Error(`the server did not log "listening on" within ${START_TIMEOUT_MS} ms:\n${lines.join("\n")}`));
    }, START_TIMEOUT_MS);

    createInterface({ input: child.stdout }).on("line", (line) => {
      lines.push(line);

      const ready = READY.exec(line);

      if (ready?.[1] !== undefined) {
        clearTimeout(timer);
        resolve(ready[1]);
      }
    });
    child.on("exit", (code) => {
      clearTimeout(timer);
      reject(new Error(`the server exited with ${String(code)} before it was ready:\n${lines.join("\n")}`));
    });
  });

  return {
    baseURL: `http://127.0.0.1:${port}`,
    build: launchedCommit(),
    log: () => lines.join("\n"),
    stop: async () => {
      await stop(child);
      rmSync(dir, { recursive: true, force: true });
    },
  };
}

async function stop(child: ChildProcess): Promise<void> {
  if (child.pid === undefined || child.exitCode !== null) {
    return;
  }

  const exited = new Promise<void>((resolve) => {
    child.once("exit", () => {
      resolve();
    });
  });

  // The negative PID signals the whole process group: `mise`, its shell and the server.
  process.kill(-child.pid, "SIGTERM");

  const timer = setTimeout(() => {
    if (child.pid !== undefined) {
      process.kill(-child.pid, "SIGKILL");
    }
  }, STOP_TIMEOUT_MS);

  await exited;
  clearTimeout(timer);
}
