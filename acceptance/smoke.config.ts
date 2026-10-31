// The checks against the deployed service, run by hand and outside the Green gate (ADR 0013).
// No global setup and no local server: they reach the service at its address, without control channels.
import { defineConfig, devices } from "@playwright/test";

import { apiPlayer, apiServer } from "./drivers/api";
import { controls } from "./drivers/controls";
import { uiPlayer } from "./drivers/ui";
import type { Drivers } from "./dsl/drivers";

const api: Drivers = { player: apiPlayer, server: apiServer, controls };
const ui: Drivers = { player: uiPlayer, server: apiServer, controls };

export default defineConfig<{ readonly drivers: Drivers }>({
  testDir: "smoke",
  forbidOnly: true,
  retries: 0,
  reporter: "list",
  use: {
    ...devices["Desktop Chrome"],
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
  },
  projects: [
    // `pnpm smoke`: every check through both drivers, unless tagged with one project's tag.
    { name: "ui", testMatch: "deployed.spec.ts", grepInvert: /@api/u, use: { drivers: ui } },
    { name: "api", testMatch: "deployed.spec.ts", grepInvert: /@ui/u, use: { drivers: api } },
    // `pnpm smoke:hold`: the page reconnects by itself, so only the API driver can tell a cut stream.
    { name: "hold", testMatch: "hold.spec.ts", use: { drivers: api } },
  ],
});
