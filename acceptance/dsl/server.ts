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
      /** Tells the time on the server's clock, an RFC 3339 time such as "2027-03-01T08:30:00Z". */
      tellTheTime: async (time: string): Promise<void> => {
        const told = await driver.time();

        expect(Date.parse(told), `the server should tell the time ${time}, but tells ${told}`).toBe(Date.parse(time));
      },
      findNothingAtAMissingTestControl: async (): Promise<void> => {
        const missing = await driver.missingTestControl();

        expect(missing, "the server should find no missing test control").toEqual(NOT_FOUND);
      },
    };
  }
}
