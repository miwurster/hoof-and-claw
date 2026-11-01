// The API driver's view of the server itself, outside any Player.
import { expect } from "@playwright/test";
import type { APIResponse } from "@playwright/test";
import type { ErrorBody, Health, ServerEvent } from "contract";

import type { Connection } from "./connection";
import { decode, FORWARDED_FOR } from "./stream";

const MISSING_GAME_ROUTE = "/api/a-route-that-does-not-exist";
const MISSING_TEST_CONTROL = "/test-controls/a-control-that-does-not-exist";

async function missing(response: APIResponse) {
  const body = await decode<ErrorBody>(response);

  return { status: response.status(), error: body.error };
}

export async function apiServer(connection: Connection) {
  const request = await connection.request.newContext({
    baseURL: connection.baseURL,
    extraHTTPHeaders: { [FORWARDED_FOR]: connection.network },
  });

  return {
    health: async () => {
      const response = await request.get("/health");

      expect(response.status(), "the health check should answer 200").toBe(200);

      const health = await decode<Health>(response);

      return { build: health.build, testControls: health.testControls };
    },
    missingGameRoute: async () => missing(await request.get(MISSING_GAME_ROUTE)),
    missingTestControl: async () => missing(await request.get(MISSING_TEST_CONTROL)),
    time: async () => {
      const response = await request.get("/api/snapshot");

      expect(response.status(), "the snapshot should answer 200").toBe(200);

      const snapshot = await decode<ServerEvent>(response);

      if (snapshot.type !== "snapshot") {
        throw new Error(`the snapshot route answered a ${snapshot.type} event`);
      }

      return snapshot.serverTime;
    },
    close: async () => {
      await request.dispose();
    },
  };
}
