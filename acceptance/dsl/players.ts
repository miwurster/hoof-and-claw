// A group of Players under load, whose steps measure (ADR 0015).
import { setTimeout as sleep } from "node:timers/promises";

import { expect } from "@playwright/test";

import type { Direction, MeasuredPlayerDriver } from "./drivers";
import type { Measurements, Metric } from "./measurements";
import type { PriceSource } from "./price-source";

/**
 * The server lets at most one price a second out to the streams, the first change after a quiet second at once
 * (ADR 0010). Waiting a little more than that second before each measured price measures only such first changes,
 * as long as the price before it went out at once too: a case sends no other price within a second before it.
 */
const QUIET_SECOND_MS = 1100;

const BYTES_PER_MB = 1024 * 1024;

/** Every Player's result of one step, and the errors among them, in glossary terms. */
async function each<Result>(
  drivers: readonly MeasuredPlayerDriver[],
  step: (driver: MeasuredPlayerDriver) => Promise<Result>,
): Promise<{ readonly results: Result[]; readonly errors: string[] }> {
  const settled = await Promise.allSettled(drivers.map((driver) => step(driver)));
  const results: Result[] = [];
  const errors: string[] = [];

  for (const [index, outcome] of settled.entries()) {
    if (outcome.status === "fulfilled") {
      results.push(outcome.value);
    } else {
      errors.push(`Player ${String(index + 1)}: ${String(outcome.reason)}`);
    }
  }

  return { results, errors };
}

/** Many Players at once, each with one stream, acting together. */
export class Players {
  readonly should;

  constructor(
    private readonly drivers: readonly MeasuredPlayerDriver[],
    private readonly measurements: Measurements,
    residentMemory: () => number,
  ) {
    this.should = {
      receiveEachPriceOnTheLastStreamWithinBudget: async (): Promise<void> => {
        await this.withinBudget("priceFanOut");
      },
      placeTheirGuessesWithinBudget: async (): Promise<void> => {
        await this.withinBudget("placingAGuess");
      },
      seeEveryGuessResolvedWithinBudget: async (): Promise<void> => {
        await this.withinBudget("resolutionBurst");
      },
      receiveTheFirstEventWithinBudget: async (): Promise<void> => {
        await this.withinBudget("firstEvent");
      },
      leaveTheServerWithinItsMemoryBudget: async (): Promise<void> => {
        this.measurements.record("residentMemory", residentMemory() / BYTES_PER_MB);
        await this.withinBudget("residentMemory");
      },
      haveNoStreamClosedByTheServer: (): Promise<void> => {
        const closed = this.streamsClosedByTheServer();

        expect(closed, `no Player's stream should be closed by the server, but ${String(closed)} were`).toBe(0);

        return Promise.resolve();
      },
    };
  }

  /** Every Player opens the game at once; measures each stream's time to its first event. */
  async openTheGame(): Promise<void> {
    const durations = await this.step((driver) => driver.opensTheGame(), "open the game");

    for (const duration of durations) {
      this.measurements.record("firstEvent", duration);
    }
  }

  /** Every Player places a Guess at once; measures each Guess placed. */
  async guess(direction: Direction): Promise<void> {
    const durations = await this.step((driver) => driver.guesses(direction), `guess ${direction}`);

    for (const duration of durations) {
      this.measurements.record("placingAGuess", duration);
    }
  }

  /** After a quiet second, the Price source sends a price; measures until it arrived on the last stream. */
  async receiveThePriceSentBy(priceSource: PriceSource, price: string): Promise<void> {
    await sleep(QUIET_SECOND_MS);

    const sent = performance.now();

    await priceSource.sends(price);

    const arrivals = await this.step((driver) => driver.receives(price), `receive the price ${price}`);

    this.measurements.record("priceFanOut", Math.max(...arrivals) - sent);
  }

  /** The Price source sends a price that resolves every due Guess; measures until the last Guess resolved event. */
  async seeTheirGuessesResolvedByThePriceSentBy(priceSource: PriceSource, price: string): Promise<void> {
    const sent = performance.now();

    await priceSource.sends(price);

    const arrivals = await this.step((driver) => driver.receivesTheResolution(), "see their Guess resolved");

    this.measurements.record("resolutionBurst", Math.max(...arrivals) - sent);
  }

  streamsClosedByTheServer(): number {
    return this.drivers.reduce((closed, driver) => closed + driver.streamsClosedByTheServer(), 0);
  }

  async leave(): Promise<void> {
    this.measurements.recordStreamsClosedByTheServer(this.streamsClosedByTheServer());
    await Promise.all(this.drivers.map((driver) => driver.close()));
  }

  /** Runs a step for every Player at once and fails on the first error, which the measurements keep. */
  private async step<Result>(step: (driver: MeasuredPlayerDriver) => Promise<Result>, what: string): Promise<Result[]> {
    const { results, errors } = await each(this.drivers, step);

    this.measurements.recordErrors(errors);
    expect(errors, `every Player should ${what} without an error`).toEqual([]);

    return results;
  }

  private withinBudget(metric: Metric): Promise<void> {
    expect(this.measurements.breach(metric), `the ${metric} budget should hold`).toBeUndefined();

    return Promise.resolve();
  }
}
