// The driver interface: what the DSL needs from a channel to the system, and nothing about how it gets it.
import type { APIRequest, Browser } from "@playwright/test";

/** Everything a driver may use to reach the server under test. */
export type Connection = {
  readonly baseURL: string;
  readonly browser: Browser;
  readonly request: APIRequest;
  /** The address the driver's requests come from, as the proxy in front of the server reports it. */
  readonly network: string;
};

/** A side a Guess takes. */
export type Direction = "Up" | "Down";

/** Why the server refuses a Guess, or a visitor; `internal` when it cannot answer, as while the store is down. */
export type Refusal =
  "openGuessExists" | "priceDelayed" | "invalidDirection" | "badRequest" | "rateLimited" | "internal";

/** The result of a Resolved guess. */
export type Outcome = "Correct" | "Wrong";

/** Whether the Open guess would be Correct if it resolved at the latest price, or neither while the price is unchanged. */
export type Standing = "Correct" | "Wrong" | "Unchanged";

/**
 * One Player's channel to the game: the page in a browser, or the HTTP API and its stream.
 * Prices are dollar strings such as "64,000.00", and `null` is no price at all.
 */
export type PlayerDriver = {
  readonly opensAnUnknownPage: () => Promise<void>;
  readonly opensTheGame: () => Promise<void>;
  /** Opens the game in one more tab with the same identity, which the answered driver acts in. */
  readonly opensAnotherTab: () => Promise<PlayerDriver>;
  /** Closes every page or stream of the game, so the Player has none open. */
  readonly closesTheGame: () => Promise<void>;
  /** Opens one more stream of the game, and answers the error code it was refused with, or `null`. */
  readonly triesToOpenTheGame: () => Promise<string | null>;
  /** Reads the snapshot alone, and answers the error code it was refused with, or `null`. */
  readonly triesToReadTheSnapshot: () => Promise<string | null>;
  /** The page asks for its stream again and again while the server refuses it. */
  readonly expectsReconnecting: () => Promise<void>;
  /** The page has its stream back, with a snapshot, after its reconnect backoff. */
  readonly expectsTheGameBack: () => Promise<void>;
  readonly losesTheNetwork: () => Promise<void>;
  readonly regainsTheNetwork: () => Promise<void>;
  readonly expectsTheGame: () => Promise<void>;
  readonly expectsPrice: (price: string | null) => Promise<void>;
  readonly expectsPriceDelayed: (delayed: boolean) => Promise<void>;
  /** Some price, whichever it is: the deployed service shows Bitstamp's. */
  readonly expectsAnyPrice: () => Promise<void>;
  /** The stream stays open for this long without the server closing it; resolves early with how long it lasted. */
  readonly holdsTheStream: (milliseconds: number) => Promise<number>;
  /** The stream brings the server's keepalive, a named event a page's `EventSource` counts as received data. */
  readonly expectsAKeepalive: () => Promise<void>;
  readonly expectsScore: (score: number) => Promise<void>;
  readonly expectsNoOpenGuess: () => Promise<void>;
  readonly expectsGuessing: (possible: boolean) => Promise<void>;
  readonly expectsAnIdentity: () => Promise<void>;
  readonly expectsAnnouncement: (announcement: "Price delayed" | "Price live") => Promise<void>;
  readonly expectsWhereTheScoreLives: () => Promise<void>;
  readonly guesses: (direction: Direction) => Promise<void>;
  /** Presses a Direction while the server's answer is held back, until `receivesTheAnswer`. */
  readonly startsToGuess: (direction: Direction) => Promise<void>;
  readonly receivesTheAnswer: () => Promise<void>;
  /** Places a Guess whose request also carries a price and a time of the Player's choosing. */
  readonly guessesWithAPriceAndTimeOfTheirOwn: (direction: Direction) => Promise<void>;
  readonly guessesADirectionThatIsNeitherUpNorDown: () => Promise<void>;
  readonly guessesWithABodyThatIsNotJson: () => Promise<void>;
  readonly guessesWithoutSayingTheBodyIsJson: () => Promise<void>;
  readonly movesTheFocusTo: (direction: Direction) => Promise<void>;
  readonly pressesEnter: () => Promise<void>;
  /** Loads the page alone, without the game's stream. */
  readonly opensThePage: () => Promise<void>;
  readonly expectsOpenGuess: (direction: Direction, startPrice: string) => Promise<void>;
  /** The whole seconds left on the Open guess, at most this many and less only by the time the check takes. */
  readonly expectsSecondsLeft: (seconds: number) => Promise<void>;
  readonly expectsDifference: (difference: string, standing: Standing) => Promise<void>;
  readonly expectsWaitingForThePriceToMove: () => Promise<void>;
  /** The result of the most recent Resolved guess is shown: its Outcome, Start price, End price and Score change. */
  readonly expectsResult: (outcome: Outcome, startPrice: string, endPrice: string) => Promise<void>;
  /** No result of a Resolved guess is shown. */
  readonly expectsNoResult: () => Promise<void>;
  /** A screen reader is told the Outcome politely, and never the running price. */
  readonly expectsOutcomeAnnounced: (outcome: Outcome) => Promise<void>;
  readonly expectsNoSecondGuess: () => Promise<void>;
  readonly expectsRefusal: (refusal: Refusal) => Promise<void>;
  readonly expectsEverythingByRoleAndName: () => Promise<void>;
  readonly expectsHornFocusRing: (direction: Direction) => Promise<void>;
  readonly expectsNoIdentity: () => Promise<void>;
  /** The game's API turns the visitor away, without an identity. */
  readonly expectsTurnedAway: (refusal: Refusal) => Promise<void>;
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

/** What the store fails: nothing, every save of resolutions, or every read and write. */
export type StoreFailure = "nothing" | "resolutions" | "everything";

/** The test controls: the Price source, the clock and the store. Prices are dollar strings such as "64,000.00". */
export type ControlsDriver = {
  readonly sendsPrice: (price: string) => Promise<void>;
  readonly setsConnected: (connected: boolean) => Promise<void>;
  readonly advancesClock: (milliseconds: number) => Promise<void>;
  readonly setsClock: (time: string) => Promise<void>;
  readonly setsStoreFailing: (failure: StoreFailure) => Promise<void>;
  /** Puts the Price source back into its start state, Price delayed and no price, and makes the store fail nothing. */
  readonly resets: () => Promise<void>;
  readonly close: () => Promise<void>;
};

/** The drivers of one Playwright project. */
export type Drivers = {
  readonly player: (connection: Connection) => Promise<PlayerDriver>;
  readonly server: (connection: Connection) => Promise<ServerDriver>;
  readonly controls: (connection: Connection) => Promise<ControlsDriver>;
};

/**
 * A Player under load, whose steps report when they happened, in milliseconds on the harness's own clock
 * (`performance.now()`), so a step that measures needs no polling.
 * Prices are dollar strings such as "64,000.00". Every step rejects on an error.
 */
export type MeasuredPlayerDriver = {
  /** Opens the game's stream and answers how long its first event took to arrive. */
  readonly opensTheGame: () => Promise<number>;
  /** Places a Guess and answers how long the server took to accept it; rejects when it is refused. */
  readonly guesses: (direction: Direction) => Promise<number>;
  /** Answers when the price first arrived on the stream, waiting for it if it has not yet. */
  readonly receives: (price: string) => Promise<number>;
  /** Answers when the event that the Player's Guess resolved arrived on the stream, waiting for it if it has not yet. */
  readonly receivesTheResolution: () => Promise<number>;
  /** How many of the Player's streams the server has closed or broken off. */
  readonly streamsClosedByTheServer: () => number;
  readonly close: () => Promise<void>;
};

/** The performance project's driver for a Player under load (ADR 0015). */
export type MeasuredPlayers = (connection: Connection) => Promise<MeasuredPlayerDriver>;
