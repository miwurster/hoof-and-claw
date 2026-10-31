import { expect } from "@playwright/test";

import type { PlayerDriver } from "./drivers";

const MINUTE_MS = 60_000;

async function keepTheStreamOpen(name: string, driver: PlayerDriver, minutes: number): Promise<void> {
  const held = await driver.holdsTheStream(minutes * MINUTE_MS);
  const closedAfter = (held / MINUTE_MS).toFixed(1);

  expect(
    held,
    `${name}'s stream should stay open for ${String(minutes)} minutes, but the server closed it after ${closedAfter} minutes`,
  ).toBeGreaterThanOrEqual(minutes * MINUTE_MS);
}

/** A Player, named by alias, acting through the project's driver. */
export class Player {
  readonly should;

  constructor(
    readonly name: string,
    private readonly driver: PlayerDriver,
  ) {
    this.should = {
      seeTheGame: async (): Promise<void> => {
        await driver.expectsTheGame();
      },
      /** Sees the latest price, live, such as "64,000.00". */
      seeThePrice: async (price: string): Promise<void> => {
        await driver.expectsPrice(price);
        await driver.expectsPriceDelayed(false);
      },
      /** Sees the last price labelled Price delayed, or no price at all with `null`. */
      seeThePriceDelayed: async (price: string | null): Promise<void> => {
        await driver.expectsPrice(price);
        await driver.expectsPriceDelayed(true);
      },
      /** Sees some live price, whichever the Price source last sent. */
      seeALivePrice: async (): Promise<void> => {
        await driver.expectsAnyPrice();
        await driver.expectsPriceDelayed(false);
      },
      /** Keeps the game's stream open for `minutes` without the server closing it. */
      keepTheStreamOpenFor: async (minutes: number): Promise<void> => {
        await keepTheStreamOpen(name, driver, minutes);
      },
      haveScore: async (score: number): Promise<void> => {
        await driver.expectsScore(score);
      },
      haveNoOpenGuess: async (): Promise<void> => {
        await driver.expectsNoOpenGuess();
      },
      beAbleToGuess: async (): Promise<void> => {
        await driver.expectsGuessing(true);
      },
      notBeAbleToGuess: async (): Promise<void> => {
        await driver.expectsGuessing(false);
      },
      beKnownByThisBrowser: async (): Promise<void> => {
        await driver.expectsAnIdentity();
      },
      hearThePriceAnnouncedAs: async (announcement: "Price delayed" | "Price live"): Promise<void> => {
        await driver.expectsAnnouncement(announcement);
      },
      beToldWhereTheScoreLives: async (): Promise<void> => {
        await driver.expectsWhereTheScoreLives();
      },
    };
  }

  async opensAnUnknownPage(): Promise<void> {
    await this.driver.opensAnUnknownPage();
  }

  async opensTheGame(): Promise<void> {
    await this.driver.opensTheGame();
  }
}
