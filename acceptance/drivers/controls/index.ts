// The controls driver: the test controls' endpoints, which set the price and the time and make the store fail.
import { expect } from "@playwright/test";
import type { APIRequest, APIResponse, Browser } from "@playwright/test";
import type { AdvanceClock, SendPrice, SetClock, SetConnection, SetStore, StoreFailure } from "contract/test-controls";

type Connection = {
  readonly baseURL: string;
  readonly browser: Browser;
  readonly request: APIRequest;
  readonly network: string;
};

/** A dollar string such as "64,000.00" in cents. */
function cents(price: string): number {
  const match = /^(\d{1,3}(?:,\d{3})*|\d+)\.(\d{2})$/u.exec(price);

  if (match?.[1] === undefined || match[2] === undefined) {
    throw new Error(`"${price}" is no price in dollars with cents, such as "64,000.00"`);
  }

  return Number(match[1].replaceAll(",", "")) * 100 + Number(match[2]);
}

function done(response: APIResponse, command: string): void {
  expect(response.status(), `the test controls should ${command}`).toBe(204);
}

export async function controls(connection: Connection) {
  const request = await connection.request.newContext({ baseURL: connection.baseURL });

  return {
    sendsPrice: async (price: string) => {
      const data: SendPrice = { price: cents(price) };

      done(await request.post("/test-controls/price", { data }), `send the price ${price}`);
    },
    setsConnected: async (connected: boolean) => {
      const data: SetConnection = { connected };

      done(await request.post("/test-controls/connection", { data }), "change the connection");
    },
    advancesClock: async (milliseconds: number) => {
      const data: AdvanceClock = { milliseconds };

      done(await request.post("/test-controls/clock/advance", { data }), "advance the clock");
    },
    setsClock: async (time: string) => {
      const data: SetClock = { time };

      done(await request.post("/test-controls/clock", { data }), `set the clock to ${time}`);
    },
    setsStoreFailing: async (fails: StoreFailure) => {
      const data: SetStore = { fails };

      done(await request.post("/test-controls/store", { data }), `make the store fail ${fails}`);
    },
    resets: async () => {
      done(await request.post("/test-controls/reset"), "reset the Price source and the store");
    },
    close: async () => {
      await request.dispose();
    },
  };
}
