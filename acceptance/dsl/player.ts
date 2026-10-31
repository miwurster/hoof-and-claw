import type { PlayerDriver } from "./drivers";

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
