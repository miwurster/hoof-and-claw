// The UI driver: a Player's page in Chromium, found by role and accessible name.
import { expect } from "@playwright/test";
import type { APIRequest, Browser } from "@playwright/test";

const UNKNOWN_PAGE = "/a/page/that/does/not/exist";

type Connection = {
  readonly baseURL: string;
  readonly browser: Browser;
  readonly request: APIRequest;
};

export async function uiPlayer(connection: Connection) {
  const context = await connection.browser.newContext({ baseURL: connection.baseURL });
  const page = await context.newPage();

  return {
    opensAnUnknownPage: async () => {
      await page.goto(UNKNOWN_PAGE);
    },
    expectsTheGame: async () => {
      await expect(page).toHaveTitle("Hoof & Claw");
      await expect(page.getByRole("heading", { level: 1, name: "Hoof & Claw" })).toBeVisible();
    },
    close: async () => {
      await context.close();
    },
  };
}
