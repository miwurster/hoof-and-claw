import { expect } from "@playwright/test";

import type { ServerDriver } from "./drivers";

const NOT_FOUND = { status: 404, error: "notFound" };

/** The game server under test, as the Operator sees it. */
export class Server {
  readonly should;

  constructor(driver: ServerDriver, build: string) {
    this.should = {
      reportTheBuildItWasLaunchedFromWithItsTestControls: async (): Promise<void> => {
        const health = await driver.health();

        expect(health, `the server should report build ${build} with its test controls`).toEqual({
          build,
          testControls: true,
        });
      },
      findNothingAtAMissingGameRoute: async (): Promise<void> => {
        const missing = await driver.missingGameRoute();

        expect(missing, "the server should find no missing game route").toEqual(NOT_FOUND);
      },
      findNothingAtAMissingTestControl: async (): Promise<void> => {
        const missing = await driver.missingTestControl();

        expect(missing, "the server should find no missing test control").toEqual(NOT_FOUND);
      },
    };
  }
}
