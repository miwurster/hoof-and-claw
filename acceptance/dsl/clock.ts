import type { ControlsDriver } from "./drivers";

/** A span of time on the server's clock. */
export type Span = { readonly milliseconds: number };

export function seconds(count: number): Span {
  return { milliseconds: count * 1000 };
}

/** The server's clock, which stands still until a test moves it (ADR 0011). */
export class Clock {
  constructor(private readonly driver: ControlsDriver) {}

  async advances(span: Span): Promise<void> {
    await this.driver.advancesClock(span.milliseconds);
  }

  /** Sets the clock to an RFC 3339 time, such as "2027-03-01T08:30:00Z". */
  async isSetTo(time: string): Promise<void> {
    await this.driver.setsClock(time);
  }
}
