import type { ControlsDriver } from "./drivers";

/** The Price source the server receives prices from, as the test controls drive it. */
export class PriceSource {
  /** The source starts Price delayed before each test; the first price a test sends connects it (ADR 0011). */
  private connected = false;

  constructor(private readonly driver: ControlsDriver) {}

  /** Sends a price in dollars with cents, such as "64,000.00", which the server receives. */
  async sends(price: string): Promise<void> {
    if (!this.connected) {
      await this.restoresTheConnection();
    }

    await this.driver.sendsPrice(price);
  }

  /** The server started again, with the Price source in its start state. */
  startsOver(): void {
    this.connected = false;
  }

  async losesTheConnection(): Promise<void> {
    this.connected = false;
    await this.driver.setsConnected(false);
  }

  async restoresTheConnection(): Promise<void> {
    this.connected = true;
    await this.driver.setsConnected(true);
  }
}
