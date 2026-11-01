import { expect } from "@playwright/test";

import type { Span } from "./clock";
import type { ControlsDriver, ServerDriver } from "./drivers";
import type { PriceSource } from "./price-source";
import type { ServerProcess, Stopped } from "./server-process";

const NOT_FOUND = { status: 404, error: "notFound" };

function expectStoppedWithin(stopped: Stopped | undefined, span: Span): void {
  const took = stopped === undefined ? "has not been stopped" : `took ${String(stopped.milliseconds)} ms`;

  expect(stopped?.killed, `the server should stop by itself, but ${took}`).toBe(false);
  expect(
    stopped?.milliseconds,
    `the server should stop within ${String(span.milliseconds)} ms, but ${took}`,
  ).toBeLessThanOrEqual(span.milliseconds);
}

/** What a restart touches besides the process: the test controls start over with the new process. */
export type Restart = {
  readonly process: ServerProcess;
  readonly controls: ControlsDriver;
  readonly priceSource: PriceSource;
};

/** The game server under test, as the Operator sees it. */
export class Server {
  readonly should;
  private stopped: Stopped | undefined;
  /** The server's time when it stopped, which its clock shows again once it runs again. */
  private time: string | undefined;

  constructor(
    private readonly driver: ServerDriver,
    build: string,
    /** Absent for the deployed service, which no test restarts. */
    private readonly restart?: Restart,
  ) {
    this.should = {
      /** Stopped on `SIGTERM` by itself within this span, without being killed. */
      haveStoppedWithin: (span: Span): Promise<void> => {
        expectStoppedWithin(this.stopped, span);

        return Promise.resolve();
      },
      reportTheBuildItWasLaunchedFromWithItsTestControls: async (): Promise<void> => {
        const health = await driver.health();

        expect(health, `the server should report build ${build} with its test controls`).toEqual({
          build,
          testControls: true,
        });
      },
      /** Reports the released commit as its build, without test controls, as the deployed service does. */
      reportTheReleasedBuild: async (): Promise<void> => {
        const health = await driver.health();

        expect(health, `the server should report the released build ${build} without test controls`).toEqual({
          build,
          testControls: false,
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

  /**
   * Receives `SIGTERM`, as on a deploy, and stops.
   * The store recovers first: the server waits for a running save of resolutions, which never ends while saves fail.
   */
  async stops(): Promise<void> {
    const restart = this.restarting();

    await restart.controls.setsStoreFailing("nothing");
    this.time = await this.driver.time();
    this.stopped = await restart.process.stops();
  }

  /**
   * Starts again on the same database, with its clock where it stood and its Price source in its start state: Price
   * delayed and no price, as a fresh connection to the Price source is (ADR 0011).
   */
  async startsAgain(): Promise<void> {
    const restart = this.restarting();

    await restart.process.startsAgain();
    restart.priceSource.startsOver();

    if (this.time !== undefined) {
      await restart.controls.setsClock(this.time);
    }
  }

  /** Stops on `SIGTERM` and starts again, as on a deploy. */
  async restarts(): Promise<void> {
    await this.stops();
    await this.startsAgain();
  }

  private restarting(): Restart {
    if (this.restart === undefined) {
      throw new Error("only a local server can be stopped and started again");
    }

    return this.restart;
  }
}

/** The store the game keeps Players, Guesses and Scores in, which the test controls can make fail. */
export type Store = ReturnType<typeof storeOf>;

/** The worker's store, made to fail through the test controls. */
export function storeOf(driver: ControlsDriver) {
  return {
    /** Every save of resolutions fails, as an unavailable store's would, until the store recovers. */
    failsResolutions: async (): Promise<void> => {
      await driver.setsStoreFailing("resolutions");
    },
    /** Every read and write fails, as an unavailable store's would, until the store recovers. */
    goesDown: async (): Promise<void> => {
      await driver.setsStoreFailing("everything");
    },
    recovers: async (): Promise<void> => {
      await driver.setsStoreFailing("nothing");
    },
  };
}
