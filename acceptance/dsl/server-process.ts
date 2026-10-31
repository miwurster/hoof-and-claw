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

/** How a server stopped on `SIGTERM`: how long it took, and whether it had to be killed. */
export type Stopped = {
  readonly milliseconds: number;
  readonly killed: boolean;
};

export type ServerProcess = {
  readonly baseURL: string;
  readonly build: string;
  readonly log: () => string;
  /** Sends `SIGTERM`, as Render does on a deploy, and keeps the database for `startsAgain`. */
  readonly stops: () => Promise<Stopped>;
  /** Starts the server again on the same port and database, as after a deploy. */
  readonly startsAgain: () => Promise<void>;
  readonly isRunning: () => boolean;
  /** The server's resident memory in bytes, as `ps` reports it. */
  readonly residentMemory: () => number;
  readonly stop: () => Promise<void>;
};

/** The commit `mise run launch` reports as the server's build. */
export function launchedCommit(): string {
  return execFileSync("git", ["rev-parse", "HEAD"], { encoding: "utf8" }).trim();
}

/** Starts a server on an empty database and resolves once it logs that it is listening. */
export async function startServer(): Promise<ServerProcess> {
  const dir = mkdtempSync(join(tmpdir(), "hoof-and-claw-"));
  const database = join(dir, "game.db");
  const lines: string[] = [];
  let launched = await launch(database, "0", lines);
  let running = true;

  return {
    baseURL: `http://127.0.0.1:${launched.port}`,
    build: launchedCommit(),
    log: () => lines.join("\n"),
    stops: () => {
      running = false;

      return stop(launched.child);
    },
    startsAgain: async () => {
      launched = await launch(database, launched.port, lines);
      running = true;
    },
    isRunning: () => running,
    residentMemory: () => residentMemory(launched.child),
    stop: async () => {
      await stop(launched.child);
      rmSync(dir, { recursive: true, force: true });
    },
  };
}

/** Launches a server on the database and port, and resolves with the port once it logs that it is listening. */
async function launch(
  database: string,
  port: string,
  lines: string[],
): Promise<{ readonly child: ChildProcess; readonly port: string }> {
  const child = spawn("mise", ["run", "--quiet", "launch"], {
    cwd: join(import.meta.dirname, "..", ".."),
    detached: true,
    env: { ...process.env, PORT: port, DATABASE_PATH: database, NO_COLOR: "1" },
    stdio: ["ignore", "pipe", "pipe"],
  });

  createInterface({ input: child.stderr }).on("line", (line) => {
    lines.push(line);
  });

  const listening = await new Promise<string>((resolve, reject) => {
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

  return { child, port: listening };
}

/** The server is the process named `server` in the process group `mise run launch` leads. */
function residentMemory(child: ChildProcess): number {
  const table = execFileSync("ps", ["-A", "-o", "pgid=,rss=,comm="], { encoding: "utf8" });

  for (const line of table.split("\n")) {
    const [, group, kilobytes] = /^\s*(\d+)\s+(\d+)\s+(?:.*\/)?server$/u.exec(line) ?? [];

    if (Number(group) === child.pid && kilobytes !== undefined) {
      return Number(kilobytes) * 1024;
    }
  }

  throw new Error(`no process named server in the process group ${String(child.pid)}`);
}

async function stop(child: ChildProcess): Promise<Stopped> {
  if (child.pid === undefined || child.exitCode !== null || child.signalCode !== null) {
    return { milliseconds: 0, killed: false };
  }

  const started = Date.now();
  let killed = false;

  const exited = new Promise<void>((resolve) => {
    child.once("exit", () => {
      resolve();
    });
  });

  // The negative PID signals the whole process group: `mise`, its shell and the server.
  process.kill(-child.pid, "SIGTERM");

  const timer = setTimeout(() => {
    if (child.pid !== undefined) {
      killed = true;
      process.kill(-child.pid, "SIGKILL");
    }
  }, STOP_TIMEOUT_MS);

  await exited;
  clearTimeout(timer);

  return { milliseconds: Date.now() - started, killed };
}
