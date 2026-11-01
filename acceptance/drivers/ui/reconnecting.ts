// The page reconnecting its stream: how often it asks, and when it has the stream back.
import { expect } from "@playwright/test";
import type { Locator, Page } from "@playwright/test";

const EVENTS = "/api/events";

/**
 * How long the page may take to ask for its stream again, or to get it back: its reconnect backoff doubles from up to
 * 1 s, so a few refused streams already mean waits of several seconds.
 */
const RECONNECT_TIMEOUT_MS = 20_000;

/** Watches a page ask for its stream. */
export class Reconnecting {
  /** How many times the page has asked for its stream. */
  private requests = 0;

  constructor(
    page: Page,
    /** The price region, which shows Price delayed until a snapshot arrives. */
    private readonly price: Locator,
  ) {
    page.on("request", (request) => {
      if (new URL(request.url()).pathname === EVENTS) {
        this.requests += 1;
      }
    });
  }

  /** Two more requests for the stream than when the check began: the page asked again, and again after a longer wait. */
  async expectsAskingAgain(): Promise<void> {
    const before = this.requests;

    await expect
      .poll(() => this.requests, {
        message: "the page should keep asking for its stream",
        timeout: RECONNECT_TIMEOUT_MS,
      })
      .toBeGreaterThanOrEqual(before + 2);
  }

  /** Only a snapshot ends Price delayed on a page that lost its stream, so a live price shows the stream is back. */
  async expectsTheStreamBack(): Promise<void> {
    await expect(
      this.price.getByText("Price delayed", { exact: true }),
      "the page should get its stream back",
    ).toHaveCount(0, { timeout: RECONNECT_TIMEOUT_MS });
  }
}
