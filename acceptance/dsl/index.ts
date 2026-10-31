// What a test case imports: the actors, and `test` that hands them out.
import { test as base } from "@playwright/test";

import { Clock } from "./clock";
import type { ControlsDriver, Drivers, PlayerDriver } from "./drivers";
import { freshNetwork, Neighbours } from "./network";
import { Player } from "./player";
import { PriceSource } from "./price-source";
import { Server } from "./server";
import { startServer } from "./server-process";
import type { ServerProcess } from "./server-process";

export { milliseconds, seconds } from "./clock";

type Actors = {
  readonly alice: Player;
  /** Another Player, on a network of his own. */
  readonly bob: Player;
  /** Players on Alice's network. */
  readonly neighbours: Neighbours;
  readonly priceSource: PriceSource;
  readonly clock: Clock;
  readonly server: Server;
};

type Infrastructure = {
  /** The address Alice and her neighbours come from. */
  readonly network: string;
  readonly controls: ControlsDriver;
  readonly serverLog: void;
};

export type Options = {
  /** The project's drivers, set in playwright.config.ts. */
  readonly drivers: Drivers;
};

type Worker = {
  readonly serverProcess: ServerProcess;
};

export const test = base.extend<Actors & Infrastructure & Options, Worker>({
  // Playwright reads an option's default from this tuple; the config sets the real drivers.
  drivers: [
    {
      player: () => Promise.reject(new Error("no player driver configured")),
      server: () => Promise.reject(new Error("no server driver configured")),
      controls: () => Promise.reject(new Error("no controls driver configured")),
    },
    { option: true },
  ],

  serverProcess: [
    // oxlint-disable-next-line no-empty-pattern -- Playwright requires an object pattern for a fixture without dependencies.
    async ({}, use) => {
      const serverProcess = await startServer();

      await use(serverProcess);
      await serverProcess.stop();
    },
    { scope: "worker" },
  ],

  // oxlint-disable-next-line no-empty-pattern -- Playwright requires an object pattern for a fixture without dependencies.
  network: async ({}, use) => {
    await use(freshNetwork());
  },

  alice: async ({ drivers, serverProcess, browser, playwright, network }, use) => {
    const driver = await drivers.player({
      baseURL: serverProcess.baseURL,
      browser,
      request: playwright.request,
      network,
    });

    await use(new Player("Alice", driver));
    await driver.close();
  },

  bob: async ({ drivers, serverProcess, browser, playwright }, use) => {
    const driver = await drivers.player({
      baseURL: serverProcess.baseURL,
      browser,
      request: playwright.request,
      network: freshNetwork(),
    });

    await use(new Player("Bob", driver));
    await driver.close();
  },

  neighbours: async ({ drivers, serverProcess, browser, playwright, network }, use) => {
    const joined: PlayerDriver[] = [];

    await use(
      new Neighbours(async (name) => {
        const driver = await drivers.player({
          baseURL: serverProcess.baseURL,
          browser,
          request: playwright.request,
          network,
        });

        joined.push(driver);

        return new Player(name, driver);
      }),
    );
    await Promise.all(joined.map((driver) => driver.close()));
  },

  // Every test starts with the Price source in its start state: Price delayed and no price (ADR 0011).
  controls: [
    async ({ drivers, serverProcess, browser, playwright }, use) => {
      const driver = await drivers.controls({
        baseURL: serverProcess.baseURL,
        browser,
        request: playwright.request,
        network: freshNetwork(),
      });

      await driver.resetsPriceSource();
      await use(driver);
      await driver.close();
    },
    { auto: true },
  ],

  priceSource: async ({ controls }, use) => {
    await use(new PriceSource(controls));
  },

  clock: async ({ controls }, use) => {
    await use(new Clock(controls));
  },

  server: async ({ drivers, serverProcess, browser, playwright }, use) => {
    const driver = await drivers.server({
      baseURL: serverProcess.baseURL,
      browser,
      request: playwright.request,
      network: freshNetwork(),
    });

    await use(new Server(driver, serverProcess.build));
    await driver.close();
  },

  // Keeps the worker's server log beside a failed test's trace and screenshot.
  serverLog: [
    async ({ serverProcess }, use, testInfo) => {
      await use();

      if (testInfo.status !== testInfo.expectedStatus) {
        await testInfo.attach("server log", { body: serverProcess.log(), contentType: "text/plain" });
      }
    },
    { auto: true },
  ],
});
