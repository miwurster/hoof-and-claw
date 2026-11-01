import { defineConfig, devices } from "@playwright/test";

import { apiPlayer, apiServer, measuredApiPlayer } from "./drivers/api";
import { controls } from "./drivers/controls";
import { uiPlayer } from "./drivers/ui";
import type { Budgets, Options } from "./dsl";
import type { Drivers } from "./dsl/drivers";

const api: Drivers = { player: apiPlayer, server: apiServer, controls };
const ui: Drivers = { player: uiPlayer, server: apiServer, controls };

/**
 * The performance budgets (ADR 0015): about ten times the expected values, so shared CI runners stay green on noise.
 * A change to a budget needs a reason in its commit.
 */
const budgets: Budgets = {
  priceFanOut: { p95: 250, max: 1000 },
  placingAGuess: { p95: 100 },
  guessBurst: { max: 1000 },
  resolutionBurst: { max: 2000 },
  firstEvent: { p95: 200 },
  residentMemory: { max: 128 },
};

export default defineConfig<Options>({
  globalSetup: "./global-setup.ts",
  forbidOnly: true,
  retries: 0,
  fullyParallel: false,
  // A failed save of resolutions is tried again after up to 5 s of real time, so a resolution may take that long to
  // show once the store recovers; every waiting assertion allows twice that.
  expect: { timeout: 10_000 },
  reporter: process.env["CI"] === undefined ? "list" : [["list"], ["html", { open: "never" }]],
  use: {
    ...devices["Desktop Chrome"],
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
  },
  projects: [
    // Every case runs through both drivers; a case tagged with one project's tag runs only there.
    { name: "ui", testDir: "cases", grepInvert: /@api/u, use: { drivers: ui } },
    { name: "api", testDir: "cases", grepInvert: /@ui/u, use: { drivers: api } },
    // Its own output folder, so a run of it alone keeps the other projects' results; CI keeps measurements.json.
    {
      name: "performance",
      testDir: "performance",
      outputDir: "test-results/performance",
      workers: 1,
      timeout: 60_000,
      use: { drivers: api, measuring: { players: measuredApiPlayer, budgets } },
    },
  ],
});
