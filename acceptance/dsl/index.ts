// What a test case imports: the actors, and `test` that hands them out.
import { test as base } from "@playwright/test";

import { Clock } from "./clock";
import type { Connection, ControlsDriver, Drivers, MeasuredPlayers, PlayerDriver } from "./drivers";
import { Measurements } from "./measurements";
import type { Budgets } from "./measurements";
import { freshNetwork, Neighbours } from "./network";
import { Player } from "./player";
import { Players } from "./players";
import { PriceSource } from "./price-source";
import { Server } from "./server";
import { startServer } from "./server-process";
import type { ServerProcess } from "./server-process";

export { milliseconds, seconds } from "./clock";

export type { Budgets } from "./measurements";

type Actors = {
  readonly alice: Player;
  /** Another Player, on a network of his own. */
  readonly bob: Player;
  /** Players on Alice's network. */
  readonly neighbours: Neighbours;
  readonly priceSource: PriceSource;
  readonly clock: Clock;
  readonly server: Server;
  /** A group of this many Players under load, each from a network of their own (ADR 0015). */
  readonly players: (count: number) => Promise<Players>;
};

type Infrastructure = {
  /** The address Alice and her neighbours come from. */
  readonly network: string;
  readonly controls: ControlsDriver;
  readonly serverLog: void;
  /** How a driver reaches the worker's server from a network. */
  readonly connect: (network: string) => Connection;
};

export type Options = {
  /** The project's drivers, set in playwright.config.ts. */
  readonly drivers: Drivers;
  /** The performance project's Players under load and the budgets they are held to, set in playwright.config.ts. */
  readonly measuring: Measuring;
};

type Measuring = {
  readonly players: MeasuredPlayers;
  readonly budgets: Budgets;
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

  measuring: [
    {
      players: () => Promise.reject(new Error("no measured player driver configured")),
      budgets: {
        priceFanOut: {},
        placingAGuess: {},
        guessBurst: {},
        resolutionBurst: {},
        firstEvent: {},
        residentMemory: {},
      },
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

  connect: async ({ serverProcess, browser, playwright }, use) => {
    await use((network) => ({ baseURL: serverProcess.baseURL, browser, request: playwright.request, network }));
  },

  // oxlint-disable-next-line no-empty-pattern -- Playwright requires an object pattern for a fixture without dependencies.
  network: async ({}, use) => {
    await use(freshNetwork());
  },

  alice: async ({ drivers, connect, network }, use) => {
    const driver = await drivers.player(connect(network));

    await use(new Player("Alice", driver));
    await driver.close();
  },

  bob: async ({ drivers, connect }, use) => {
    const driver = await drivers.player(connect(freshNetwork()));

    await use(new Player("Bob", driver));
    await driver.close();
  },

  neighbours: async ({ drivers, connect, network }, use) => {
    const joined: PlayerDriver[] = [];

    await use(
      new Neighbours(async (name) => {
        const driver = await drivers.player(connect(network));

        joined.push(driver);

        return new Player(name, driver);
      }),
    );
    await Promise.all(joined.map((driver) => driver.close()));
  },

  // Every test starts with the Price source in its start state: Price delayed and no price (ADR 0011).
  controls: [
    async ({ drivers, connect }, use) => {
      const driver = await drivers.controls(connect(freshNetwork()));

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

  server: async ({ drivers, serverProcess, connect, controls, priceSource }, use) => {
    const driver = await drivers.server(connect(freshNetwork()));

    const server = new Server(driver, serverProcess.build, { process: serverProcess, controls, priceSource });

    await use(server);

    // The next test needs the worker's server, even when this one stopped it and failed.
    if (!serverProcess.isRunning()) {
      await server.startsAgain();
    }

    await driver.close();
  },

  // Every case writes its numbers to measurements.json in its output folder, which CI keeps as an artefact.
  players: async ({ measuring, serverProcess, connect }, use, testInfo) => {
    const measurements = new Measurements(measuring.budgets);
    const groups: Players[] = [];

    await use(async (count) => {
      const drivers = await Promise.all(
        Array.from({ length: count }, () => measuring.players(connect(freshNetwork()))),
      );

      const group = new Players(drivers, measurements, serverProcess.residentMemory);

      groups.push(group);

      return group;
    });

    await Promise.all(groups.map((group) => group.leave()));

    await measurements.keep(testInfo);
    measurements.destroy();
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
