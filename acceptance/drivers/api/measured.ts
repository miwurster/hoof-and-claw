// The API driver for a Player under load: one stream and timed steps, timestamped as each event arrives (ADR 0015).
import type { ErrorBody, PlaceGuess, ServerEvent } from "contract";

import type { Connection } from "./connection";
import { cents, decode, FORWARDED_FOR, read, wire } from "./stream";

const IDENTITY = "__Host-player";

/** How long a step waits for an event before it counts as an error. */
const EVENT_TIMEOUT_MS = 10_000;

/** A Player with one stream, who notes when each event arrived on the harness's clock. */
class MeasuredApiPlayer {
  private readonly streams = new AbortController();
  private reading: Promise<void> = Promise.resolve();
  private readonly priceArrivals = new Map<number, number>();
  private firstEvent: number | undefined;
  private resolution: number | undefined;
  private closedByTheServer = 0;
  private listeners: (() => void)[] = [];
  private identity: string | undefined;

  constructor(
    private readonly baseURL: string,
    private readonly network: string,
  ) {}

  async opensTheGame(): Promise<number> {
    const opened = performance.now();

    const response = await fetch(new URL("/api/events", this.baseURL), {
      headers: this.headers(),
      signal: this.streams.signal,
    });

    if (response.status !== 200 || response.body === null) {
      throw new Error(`the stream should open, but the server answered ${String(response.status)}`);
    }

    this.keepIdentity(response);
    this.reading = this.follow(response.body);

    const first = await this.until(() => this.firstEvent, "the stream's first event");

    return first - opened;
  }

  async guesses(direction: "Up" | "Down"): Promise<number> {
    const command: PlaceGuess = { direction: wire(direction) };
    const started = performance.now();

    const response = await fetch(new URL("/api/guesses", this.baseURL), {
      method: "POST",
      headers: { ...this.headers(), "Content-Type": "application/json" },
      body: JSON.stringify(command),
    });

    if (response.status !== 201) {
      throw new Error(`the Guess should be placed, but was refused as ${(await decode<ErrorBody>(response)).error}`);
    }

    await response.arrayBuffer();

    const placed = performance.now() - started;

    this.keepIdentity(response);

    return placed;
  }

  receives(price: string): Promise<number> {
    const wanted = cents(price);

    return this.until(() => this.priceArrivals.get(wanted), `the price ${price}`);
  }

  receivesTheResolution(): Promise<number> {
    return this.until(() => this.resolution, "the Guess resolved event");
  }

  streamsClosedByTheServer(): number {
    return this.closedByTheServer;
  }

  async close(): Promise<void> {
    this.streams.abort();
    await this.reading;
  }

  private headers(): Record<string, string> {
    const cookie = this.identity?.split(";")[0];

    return cookie === undefined ? { [FORWARDED_FOR]: this.network } : { [FORWARDED_FOR]: this.network, cookie };
  }

  private keepIdentity(response: Response): void {
    this.identity =
      response.headers.getSetCookie().find((candidate) => candidate.startsWith(`${IDENTITY}=`)) ?? this.identity;
  }

  /** Answers the moment `arrived` reports, as soon as an event makes it known, or rejects after a timeout. */
  private until(arrived: () => number | undefined, what: string): Promise<number> {
    const known = arrived();

    if (known !== undefined) {
      return Promise.resolve(known);
    }

    return new Promise<number>((resolve, reject) => {
      const timer = setTimeout(() => {
        this.listeners = this.listeners.filter((other) => other !== listener);
        reject(new Error(`${what} did not arrive within ${String(EVENT_TIMEOUT_MS)} ms`));
      }, EVENT_TIMEOUT_MS);

      const listener = (): void => {
        const moment = arrived();

        if (moment !== undefined) {
          clearTimeout(timer);
          this.listeners = this.listeners.filter((other) => other !== listener);
          resolve(moment);
        }
      };

      this.listeners.push(listener);
    });
  }

  private async follow(body: ReadableStream<Uint8Array>): Promise<void> {
    try {
      await read(body, (event) => {
        this.noteArrival(event, performance.now());
      });
    } catch {
      // A stream broken off counts as closed by the server below, unless the Player aborted it.
    }

    if (!this.streams.signal.aborted) {
      this.closedByTheServer += 1;
    }
  }

  private noteArrival(event: ServerEvent, now: number): void {
    this.firstEvent ??= now;

    if (event.type === "guessResolved") {
      this.resolution ??= now;
    } else if (event.type !== "guessPlaced" && event.price !== null && !this.priceArrivals.has(event.price)) {
      this.priceArrivals.set(event.price, now);
    }

    for (const listener of this.listeners) {
      listener();
    }
  }
}

export function measuredApiPlayer(connection: Connection): Promise<MeasuredApiPlayer> {
  return Promise.resolve(new MeasuredApiPlayer(connection.baseURL, connection.network));
}
