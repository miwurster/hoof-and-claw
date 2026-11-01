import type { ControlsDriver } from "./drivers";

/** The store the game keeps Players, Guesses and Scores in, which the test controls can make fail. */
export class Store {
  constructor(private readonly driver: ControlsDriver) {}

  /** Every save of resolutions fails, as an unavailable store's would, until the store recovers. */
  async failsResolutions(): Promise<void> {
    await this.driver.setsStoreFailing("resolutions");
  }

  /** Every read and write fails, as an unavailable store's would, until the store recovers. */
  async goesDown(): Promise<void> {
    await this.driver.setsStoreFailing("everything");
  }

  async recovers(): Promise<void> {
    await this.driver.setsStoreFailing("nothing");
  }
}
