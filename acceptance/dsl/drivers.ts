// The driver interface: what the DSL needs from a channel to the system, and nothing about how it gets it.
import type { APIRequest, Browser } from "@playwright/test";

/** Everything a driver may use to reach the server under test. */
export type Connection = {
  readonly baseURL: string;
  readonly browser: Browser;
  readonly request: APIRequest;
};

/** One Player's channel to the game: the page in a browser, or the HTTP API and its stream. */
export type PlayerDriver = {
  readonly opensAnUnknownPage: () => Promise<void>;
  readonly expectsTheGame: () => Promise<void>;
  readonly close: () => Promise<void>;
};

/** What the server reports about itself over HTTP. */
export type Health = {
  readonly build: string;
  readonly testControls: boolean;
};

/** The answer to a route that does not exist. */
export type Missing = {
  readonly status: number;
  readonly error: string;
};

/** The server's own HTTP surface, outside any Player. */
export type ServerDriver = {
  readonly health: () => Promise<Health>;
  readonly missingGameRoute: () => Promise<Missing>;
  readonly missingTestControl: () => Promise<Missing>;
  readonly close: () => Promise<void>;
};

/** The drivers of one Playwright project. */
export type Drivers = {
  readonly player: (connection: Connection) => Promise<PlayerDriver>;
  readonly server: (connection: Connection) => Promise<ServerDriver>;
};
