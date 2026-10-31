import { defineConfig, devices } from "@playwright/test";

import { apiPlayer, apiServer } from "./drivers/api";
import { controls } from "./drivers/controls";
import { uiPlayer } from "./drivers/ui";
import type { Options } from "./dsl";
import type { Drivers } from "./dsl/drivers";

const api: Drivers = { player: apiPlayer, server: apiServer, controls };
const ui: Drivers = { player: uiPlayer, server: apiServer, controls };

export default defineConfig<Options>({
  globalSetup: "./global-setup.ts",
  forbidOnly: true,
  retries: 0,
  fullyParallel: false,
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
    { name: "performance", testDir: "performance", workers: 1, use: { drivers: api } },
  ],
});
