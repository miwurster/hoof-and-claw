// The UI driver: a Player's page in Chromium, found by role and accessible name.
import { expect } from "@playwright/test";
import type { APIRequest, Browser, BrowserContext, Page } from "@playwright/test";

const UNKNOWN_PAGE = "/a/page/that/does/not/exist";
const IDENTITY = "__Host-player";

type Connection = {
  readonly baseURL: string;
  readonly browser: Browser;
  readonly request: APIRequest;
};

/** A dollar string such as "64,000.00" as the page shows it, or the dash for no price. */
function shown(price: string | null): string {
  return price === null ? "—" : `$${price}`;
}

/** A Player's page in their own browser context. */
class UiPlayer {
  private readonly price;
  private readonly announcement;

  constructor(
    private readonly context: BrowserContext,
    private readonly page: Page,
  ) {
    this.price = page.getByRole("region", { name: "BTC/USD price" });
    this.announcement = this.price.getByRole("status");
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

  holdsTheStream(): Promise<number> {
    return Promise.reject(new Error("the page reconnects its stream by itself; hold the stream through the API"));
  }

  async expectsScore(score: number): Promise<void> {
    const text = score < 0 ? `Score −${String(-score)}` : `Score ${String(score)}`;

    await expect(this.page.getByRole("region", { name: "Score" })).toHaveText(text);
  }

  async expectsNoOpenGuess(): Promise<void> {
    await expect(this.page.getByRole("region", { name: "Score" })).toBeVisible();
    await expect(this.page.getByRole("region", { name: "Open guess" })).toHaveCount(0);
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
  const context = await connection.browser.newContext({ baseURL: connection.baseURL });

  return new UiPlayer(context, await context.newPage());
}
