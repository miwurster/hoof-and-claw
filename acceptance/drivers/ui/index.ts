// The UI driver: a Player's page in Chromium, found by role and accessible name.
import { readFileSync } from "node:fs";
import { join } from "node:path";

import { expect } from "@playwright/test";
import type { APIRequest, Browser, BrowserContext, Locator, Page, Route } from "@playwright/test";

import { WithoutTheApi } from "./without-the-api";

const UNKNOWN_PAGE = "/a/page/that/does/not/exist";
const IDENTITY = "__Host-player";
const GUESSES = "**/api/guesses";

/** The design token each standing of the difference is coloured with (docs/design/visual-direction.md). */
const STANDING_COLOUR = { Correct: "--color-pasture", Wrong: "--color-rust", Unchanged: "--color-flint" } as const;

/** The one refusal the page tells the Player about; for the others the page already shows why. */
const RATE_LIMITED = "Too many guesses from your network. Try again in a minute.";

type Connection = {
  readonly baseURL: string;
  readonly browser: Browser;
  readonly request: APIRequest;
  readonly network: string;
};

/** The design tokens' stylesheet, the one place that names the page's colours. */
const TOKENS = readFileSync(join(import.meta.dirname, "..", "..", "..", "spa", "src", "styles", "tokens.css"), "utf8");

/** A colour token as `getComputedStyle` writes it, such as `rgb(212, 178, 106)`. */
function colour(token: string): string {
  const hex = new RegExp(`${token}:\\s*#([0-9a-f]{6});`, "iu").exec(TOKENS)?.[1];

  if (hex === undefined) {
    throw new Error(`the token stylesheet has no colour ${token}`);
  }

  const [red, green, blue] = [0, 2, 4].map((at) => Number.parseInt(hex.slice(at, at + 2), 16));

  return `rgb(${String(red)}, ${String(green)}, ${String(blue)})`;
}

/** A dollar string such as "64,000.00" as the page shows it, or the dash for no price. */
function shown(price: string | null): string {
  return price === null ? "—" : `$${price}`;
}

/** A Player's page in their own browser context. */
class UiPlayer extends WithoutTheApi {
  private readonly price;
  private readonly announcement;
  private readonly openGuess;
  private readonly result;
  private held: Promise<Route> | undefined;

  constructor(
    private readonly context: BrowserContext,
    private readonly page: Page,
  ) {
    super();
    this.price = page.getByRole("region", { name: "BTC/USD price" });
    this.announcement = this.price.getByRole("status");
    this.openGuess = page.getByRole("region", { name: "Open guess" });
    this.result = page.getByRole("region", { name: "Resolved guess" });
  }

  private button(direction: "Up" | "Down"): Locator {
    return this.page.getByRole("button", { name: direction, exact: true });
  }

  async opensThePage(): Promise<void> {
    await this.page.goto("/");
  }

  /** Presses a Direction and waits for the server's answer, so the Guess is placed or refused when this returns. */
  async guesses(direction: "Up" | "Down"): Promise<void> {
    await Promise.all([this.page.waitForResponse(GUESSES), this.button(direction).click()]);
  }

  async startsToGuess(direction: "Up" | "Down"): Promise<void> {
    let hold: ((route: Route) => void) | undefined;

    this.held = new Promise<Route>((resolve) => {
      hold = resolve;
    });
    await this.page.route(
      GUESSES,
      (route) => {
        hold?.(route);
      },
      { times: 1 },
    );
    await this.button(direction).click();
    await this.held;
  }

  async receivesTheAnswer(): Promise<void> {
    const route = await this.held;

    await route?.continue();
    this.held = undefined;
  }

  async movesTheFocusTo(direction: "Up" | "Down"): Promise<void> {
    const button = this.button(direction);

    const focused = button.and(this.page.locator(":focus"));

    for (let presses = 0; presses < 20 && (await focused.count()) === 0; presses += 1) {
      await this.page.keyboard.press("Tab");
    }

    await expect(button, `${direction} should be reachable with Tab`).toBeFocused();
  }

  async pressesEnter(): Promise<void> {
    await this.page.keyboard.press("Enter");
  }

  /** Leaves the game for a blank page, which closes the game's stream. */
  async closesTheGame(): Promise<void> {
    await this.page.goto("about:blank");
  }

  async opensAnUnknownPage(): Promise<void> {
    await this.page.goto(UNKNOWN_PAGE);
  }

  async opensTheGame(): Promise<void> {
    await this.page.goto("/");
  }

  async expectsTheGame(): Promise<void> {
    await expect(this.page).toHaveTitle("Hoof & Claw");
    await expect(this.page.getByRole("heading", { level: 1, name: "Hoof & Claw" })).toBeVisible();
  }

  async expectsPrice(expected: string | null): Promise<void> {
    await expect(this.price.getByText(shown(expected), { exact: true })).toBeVisible();
  }

  async expectsPriceDelayed(delayed: boolean): Promise<void> {
    const label = this.price.getByText("Price delayed", { exact: true });

    await (delayed ? expect(label).toBeVisible() : expect(label).toHaveCount(0));
  }

  async expectsAnyPrice(): Promise<void> {
    await expect(this.price.getByText(/^\$\d{1,3}(?:,\d{3})*\.\d{2}$/u)).toBeVisible();
  }

  async expectsScore(score: number): Promise<void> {
    const text = score < 0 ? `Score −${String(-score)}` : `Score ${String(score)}`;

    await expect(this.page.getByRole("region", { name: "Score" })).toHaveText(text);
  }

  async expectsNoOpenGuess(): Promise<void> {
    await expect(this.page.getByRole("region", { name: "Score" })).toBeVisible();
    await expect(this.page.getByRole("region", { name: "Open guess" })).toHaveCount(0);
  }

  async expectsOpenGuess(direction: "Up" | "Down", startPrice: string): Promise<void> {
    await expect(this.openGuess).toContainText(direction);
    await expect(this.openGuess).toContainText(`Start price ${shown(startPrice)}`);
  }

  /** The page counts down on its own clock, so the seconds may have moved on by a few by the time they are read. */
  async expectsSecondsLeft(seconds: number): Promise<void> {
    const left = async () => Number(/(\d+) seconds left/u.exec((await this.openGuess.textContent()) ?? "")?.[1]);
    const message = `the Open guess should show about ${String(seconds)} seconds left`;

    await expect.poll(left, message).toBeLessThanOrEqual(seconds);
    await expect.poll(left, message).toBeGreaterThan(seconds - 3);
  }

  async expectsDifference(difference: string, standing: "Correct" | "Wrong" | "Unchanged"): Promise<void> {
    const shownDifference = this.openGuess.getByText(difference, { exact: true });

    await expect(shownDifference).toBeVisible();
    await expect(shownDifference, `the difference should be coloured as ${standing}`).toHaveCSS(
      "color",
      colour(STANDING_COLOUR[standing]),
    );
  }

  async expectsWaitingForThePriceToMove(): Promise<void> {
    await expect(this.openGuess).toContainText("Waiting for the price to move…");
  }

  async expectsResult(outcome: "Correct" | "Wrong", startPrice: string, endPrice: string): Promise<void> {
    await expect(this.result).toContainText(outcome);
    await expect(this.result).toContainText(`Start price ${shown(startPrice)}`);
    await expect(this.result).toContainText(`End price ${shown(endPrice)}`);
    await expect(this.result).toContainText(outcome === "Correct" ? "Score +1" : "Score −1");
    await expect(this.openGuess).toHaveCount(0);
  }

  async expectsNoResult(): Promise<void> {
    await expect(this.page.getByRole("region", { name: "Score" })).toBeVisible();
    await expect(this.result).toHaveCount(0);
  }

  /** A polite live region tells the Outcome, and no live region holds a price. */
  async expectsOutcomeAnnounced(outcome: "Correct" | "Wrong"): Promise<void> {
    const live = this.page.locator('[aria-live="polite"], [role="status"], output');

    await expect(live.filter({ hasText: outcome }), `the Outcome ${outcome} should be announced`).toHaveCount(1);
    await expect(live.filter({ hasText: outcome })).toHaveAttribute("aria-live", "polite");
    await expect(live.filter({ hasText: "$" }), "no price should be announced").toHaveCount(0);
  }

  async expectsNoSecondGuess(): Promise<void> {
    await expect(this.openGuess).toBeVisible();
    await this.expectsGuessing(false);
  }

  async expectsRefusal(refusal: string): Promise<void> {
    if (refusal !== "rateLimited") {
      throw new Error(`the page shows no message for a Guess refused as ${refusal}`);
    }

    await expect(this.page.getByText(RATE_LIMITED, { exact: true })).toBeVisible();
  }

  async expectsEverythingByRoleAndName(): Promise<void> {
    await expect(this.button("Up")).toBeVisible();
    await expect(this.button("Down")).toBeVisible();
    await expect(this.page.getByRole("region", { name: "Score" })).toBeVisible();
    await expect(this.price).toBeVisible();
    await expect(this.openGuess).toBeVisible();
  }

  async expectsHornFocusRing(direction: "Up" | "Down"): Promise<void> {
    const button = this.button(direction);

    await expect(button).toBeFocused();
    await expect(button, "the focus ring should show").toHaveCSS("outline-style", "solid");
    await expect(button, "the focus ring should show").not.toHaveCSS("outline-width", "0px");
    await expect(button, "the focus ring should be Horn").toHaveCSS("outline-color", colour("--color-horn"));
  }

  async expectsNoIdentity(): Promise<void> {
    const cookies = await this.context.cookies();

    expect(cookies.find((cookie) => cookie.name === IDENTITY)).toBeUndefined();
  }

  async expectsGuessing(possible: boolean): Promise<void> {
    for (const name of ["Up", "Down"]) {
      const button = this.page.getByRole("button", { name, exact: true });

      await (possible ? expect(button).toBeEnabled() : expect(button).toBeDisabled());
    }
  }

  async expectsAnIdentity(): Promise<void> {
    await expect
      .poll(async () => {
        const cookies = await this.context.cookies();
        const cookie = cookies.find((candidate) => candidate.name === IDENTITY);

        return cookie === undefined
          ? undefined
          : { path: cookie.path, httpOnly: cookie.httpOnly, secure: cookie.secure, sameSite: cookie.sameSite };
      }, "the browser should hold an identity cookie")
      .toEqual({ path: "/", httpOnly: true, secure: true, sameSite: "Lax" });
  }

  async expectsAnnouncement(expected: "Price delayed" | "Price live"): Promise<void> {
    await expect(this.announcement).toHaveText(expected);
  }

  async expectsWhereTheScoreLives(): Promise<void> {
    const rules = this.page.getByText("Guess where BTC goes in the next minute. Right +1, wrong −1.");
    const where = this.page.getByText("Your score lives in this browser.");

    await expect(where).toBeVisible();

    const rulesBox = await rules.boundingBox();
    const whereBox = await where.boundingBox();

    expect(whereBox?.y ?? 0, "the line should sit beneath the rules line").toBeGreaterThan(rulesBox?.y ?? Infinity);
  }

  async close(): Promise<void> {
    await this.context.close();
  }
}

export async function uiPlayer(connection: Connection) {
  const context = await connection.browser.newContext({
    baseURL: connection.baseURL,
    extraHTTPHeaders: { "X-Forwarded-For": connection.network },
  });

  return new UiPlayer(context, await context.newPage());
}
