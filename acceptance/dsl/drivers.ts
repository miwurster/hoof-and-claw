// The driver interface: what the DSL needs from a channel to the system, and nothing about how it gets it.
import type { APIRequest, Browser } from "@playwright/test";

/** Everything a driver may use to reach the server under test. */
export type Connection = {
  readonly baseURL: string;
  readonly browser: Browser;
  readonly request: APIRequest;
};

/**
 * One Player's channel to the game: the page in a browser, or the HTTP API and its stream.
 * Prices are dollar strings such as "64,000.00", and `null` is no price at all.
 */
export type PlayerDriver = {
  readonly opensAnUnknownPage: () => Promise<void>;
  readonly opensTheGame: () => Promise<void>;
  readonly expectsTheGame: () => Promise<void>;
  readonly expectsPrice: (price: string | null) => Promise<void>;
  readonly expectsPriceDelayed: (delayed: boolean) => Promise<void>;
  /** Some price, whichever it is: the deployed service shows Bitstamp's. */
  readonly expectsAnyPrice: () => Promise<void>;
  /** The stream stays open for this long without the server closing it; resolves early with how long it lasted. */
  readonly holdsTheStream: (milliseconds: number) => Promise<number>;
  readonly expectsScore: (score: number) => Promise<void>;
  readonly expectsNoOpenGuess: () => Promise<void>;
  readonly expectsGuessing: (possible: boolean) => Promise<void>;
  readonly expectsAnIdentity: () => Promise<void>;
  readonly expectsAnnouncement: (announcement: "Price delayed" | "Price live") => Promise<void>;
  readonly expectsWhereTheScoreLives: () => Promise<void>;
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
  /** The server's time, an RFC 3339 timestamp. */
  readonly time: () => Promise<string>;
  readonly close: () => Promise<void>;
};

/** The test controls: the Price source and the clock. Prices are dollar strings such as "64,000.00". */
export type ControlsDriver = {
  readonly sendsPrice: (price: string) => Promise<void>;
  readonly setsConnected: (connected: boolean) => Promise<void>;
  readonly advancesClock: (milliseconds: number) => Promise<void>;
  readonly setsClock: (time: string) => Promise<void>;
  /** Puts the Price source back into its start state: Price delayed and no price. */
  readonly resetsPriceSource: () => Promise<void>;
  readonly close: () => Promise<void>;
};

/** The drivers of one Playwright project. */
export type Drivers = {
  readonly player: (connection: Connection) => Promise<PlayerDriver>;
  readonly server: (connection: Connection) => Promise<ServerDriver>;
  readonly controls: (connection: Connection) => Promise<ControlsDriver>;
};
