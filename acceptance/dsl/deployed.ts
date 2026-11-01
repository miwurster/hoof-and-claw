// What a check against the deployed service imports: Alice, Bob and the server, with no control channels (ADR 0011, ADR 0013).
import { execFileSync } from "node:child_process";

import { test as base } from "@playwright/test";

import type { Drivers } from "./drivers";
import { freshNetwork } from "./network";
import { Player } from "./player";
import { Server } from "./server";

/** The deployed service, or another address in SMOKE_URL. */
export const deployedURL = process.env["SMOKE_URL"] ?? "https://hoof-and-claw.miwurster.com";

/** How long `pnpm smoke:hold` holds the stream open, two hours unless SMOKE_HOLD_MINUTES says otherwise. */
export const holdMinutes = Number(process.env["SMOKE_HOLD_MINUTES"] ?? "120");

/** The commit `release` points to on origin, which Render deployed, unless RELEASED_COMMIT names another. */
function releasedCommit(): string {
  const named = process.env["RELEASED_COMMIT"];

  if (named !== undefined) {
    return named;
  }

  const line = execFileSync("git", ["ls-remote", "--exit-code", "origin", "refs/heads/release"], { encoding: "utf8" });

  return line.split("\t")[0] ?? "";
}

type Actors = {
  readonly alice: Player;
  /** A second Player on this machine, whose requests claim another network in `X-Forwarded-For`. */
  readonly bob: Player;
  readonly server: Server;
};

type Options = {
  /** The project's drivers, set in smoke.config.ts. */
  readonly drivers: Drivers;
};

export const test = base.extend<Actors & Options>({
  // Playwright reads an option's default from this tuple; the config sets the real drivers.
  drivers: [
    {
      player: () => Promise.reject(new Error("no player driver configured")),
      server: () => Promise.reject(new Error("no server driver configured")),
      controls: () => Promise.reject(new Error("the deployed service has no test controls")),
    },
    { option: true },
  ],

  alice: async ({ drivers, browser, playwright }, use) => {
    const driver = await drivers.player({
      baseURL: deployedURL,
      browser,
      request: playwright.request,
      network: freshNetwork(),
    });

    await use(new Player("Alice", driver));
    await driver.close();
  },

  bob: async ({ drivers, browser, playwright }, use) => {
    const driver = await drivers.player({
      baseURL: deployedURL,
      browser,
      request: playwright.request,
      network: freshNetwork(),
    });

    await use(new Player("Bob", driver));
    await driver.close();
  },

  server: async ({ drivers, browser, playwright }, use) => {
    const driver = await drivers.server({
      baseURL: deployedURL,
      browser,
      request: playwright.request,
      network: freshNetwork(),
    });

    await use(new Server(driver, releasedCommit()));
    await driver.close();
  },
});
