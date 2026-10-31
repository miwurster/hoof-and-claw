import { expect } from "@playwright/test";

import type { Direction, PlayerDriver, Refusal, Standing } from "./drivers";

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
    this.should = { ...seeing(name, driver), ...acting(driver) };
  }

  async opensAnUnknownPage(): Promise<void> {
    await this.driver.opensAnUnknownPage();
  }

  async opensTheGame(): Promise<void> {
    await this.driver.opensTheGame();
  }

  async opensThePage(): Promise<void> {
    await this.driver.opensThePage();
  }

  async guesses(direction: Direction): Promise<void> {
    await this.driver.guesses(direction);
  }

  /** Presses a Direction, and the server's answer is held back until `receivesTheAnswer`. */
  async startsToGuess(direction: Direction): Promise<void> {
    await this.driver.startsToGuess(direction);
  }

  async receivesTheAnswer(): Promise<void> {
    await this.driver.receivesTheAnswer();
  }

  async guessesWithAPriceAndTimeOfTheirOwn(direction: Direction): Promise<void> {
    await this.driver.guessesWithAPriceAndTimeOfTheirOwn(direction);
  }

  async guessesADirectionThatIsNeitherUpNorDown(): Promise<void> {
    await this.driver.guessesADirectionThatIsNeitherUpNorDown();
  }

  async guessesWithABodyThatIsNotJson(): Promise<void> {
    await this.driver.guessesWithABodyThatIsNotJson();
  }

  async guessesWithoutSayingTheBodyIsJson(): Promise<void> {
    await this.driver.guessesWithoutSayingTheBodyIsJson();
  }

  async movesTheFocusTo(direction: Direction): Promise<void> {
    await this.driver.movesTheFocusTo(direction);
  }

  async pressesEnter(): Promise<void> {
    await this.driver.pressesEnter();
  }
}

/** What a Player should see of the game. */
function seeing(name: string, driver: PlayerDriver) {
  return {
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
    /** Has an Open guess in this Direction from this Start price, such as "64,000.00". */
    haveAnOpenGuess: async (direction: Direction, startPrice: string): Promise<void> => {
      await driver.expectsOpenGuess(direction, startPrice);
    },
    seeSecondsLeft: async (seconds: number): Promise<void> => {
      await driver.expectsSecondsLeft(seconds);
    },
    /** Sees the difference of the latest price from the Start price, such as "+$12.40 ↑", and how it stands. */
    seeTheDifference: async (difference: string, standing: Standing): Promise<void> => {
      await driver.expectsDifference(difference, standing);
    },
    seeTheGuessWaitingForThePriceToMove: async (): Promise<void> => {
      await driver.expectsWaitingForThePriceToMove();
    },
  };
}

/** What a Player should be able to do, and what the game should know of them. */
function acting(driver: PlayerDriver) {
  return {
    beAbleToGuess: async (): Promise<void> => {
      await driver.expectsGuessing(true);
    },
    notBeAbleToGuess: async (): Promise<void> => {
      await driver.expectsGuessing(false);
    },
    notBeAbleToPlaceAnotherGuess: async (): Promise<void> => {
      await driver.expectsNoSecondGuess();
    },
    haveTheGuessRefusedAs: async (refusal: Refusal): Promise<void> => {
      await driver.expectsRefusal(refusal);
    },
    findEverythingByRoleAndName: async (): Promise<void> => {
      await driver.expectsEverythingByRoleAndName();
    },
    seeAHornFocusRingOn: async (direction: Direction): Promise<void> => {
      await driver.expectsHornFocusRing(direction);
    },
    beKnownByThisBrowser: async (): Promise<void> => {
      await driver.expectsAnIdentity();
    },
    notBeKnownByThisBrowser: async (): Promise<void> => {
      await driver.expectsNoIdentity();
    },
    beTurnedAwayAs: async (refusal: Refusal): Promise<void> => {
      await driver.expectsTurnedAway(refusal);
    },
    hearThePriceAnnouncedAs: async (announcement: "Price delayed" | "Price live"): Promise<void> => {
      await driver.expectsAnnouncement(announcement);
    },
    beToldWhereTheScoreLives: async (): Promise<void> => {
      await driver.expectsWhereTheScoreLives();
    },
  };
}
