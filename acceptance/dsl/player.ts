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
    };
  }

  async opensAnUnknownPage(): Promise<void> {
    await this.driver.opensAnUnknownPage();
  }
}
