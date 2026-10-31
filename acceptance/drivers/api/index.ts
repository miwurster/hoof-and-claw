// The API driver: the game's HTTP API and its stream, as the SPA uses them.
import { expect } from "@playwright/test";
import type { APIRequest, APIResponse, Browser } from "@playwright/test";
import type { ErrorBody, Health } from "contract";

const UNKNOWN_PAGE = "/a/page/that/does/not/exist";
const MISSING_GAME_ROUTE = "/api/a-route-that-does-not-exist";
const MISSING_TEST_CONTROL = "/test-controls/a-control-that-does-not-exist";

type Connection = {
  readonly baseURL: string;
  readonly browser: Browser;
  readonly request: APIRequest;
};

/** Decodes a body as the contract's type; the one place this driver trusts the server's JSON. */
async function decode<Body>(response: APIResponse): Promise<Body> {
  // SAFETY: the server and this driver are built from the same commit, so the body is the contract's type (ADR 0014).
  // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- Playwright types every JSON body as any.
  return (await response.json()) as Body;
}

async function missing(response: APIResponse) {
  const body = await decode<ErrorBody>(response);

  return { status: response.status(), error: body.error };
}

export async function apiPlayer(connection: Connection) {
  const request = await connection.request.newContext({ baseURL: connection.baseURL });
  let lastPage: APIResponse | undefined;

  return {
    opensAnUnknownPage: async () => {
      lastPage = await request.get(UNKNOWN_PAGE);
    },
    expectsTheGame: async () => {
      expect(lastPage?.status(), "the page should load").toBe(200);
      expect(lastPage?.headers()["content-type"]).toContain("text/html");
      expect(await lastPage?.text()).toContain("<title>Hoof &amp; Claw</title>");
    },
    close: async () => {
      await request.dispose();
    },
  };
}

export async function apiServer(connection: Connection) {
  const request = await connection.request.newContext({ baseURL: connection.baseURL });

  return {
    health: async () => {
      const response = await request.get("/health");

      expect(response.status(), "the health check should answer 200").toBe(200);

      const health = await decode<Health>(response);

      return { build: health.build, testControls: health.testControls };
    },
    missingGameRoute: async () => missing(await request.get(MISSING_GAME_ROUTE)),
    missingTestControl: async () => missing(await request.get(MISSING_TEST_CONTROL)),
    close: async () => {
      await request.dispose();
    },
  };
}
