// The API driver: the game's HTTP API and its stream, as the SPA uses them.
import { expect } from "@playwright/test";
import type { APIRequest, APIRequestContext, APIResponse, Browser } from "@playwright/test";
import type { ErrorBody, ErrorCode, OpenGuess, PlaceGuess } from "contract";

import { apply, cents, decode, FORWARDED_FOR, heldOpen, outcome, read, shownResult, wire } from "./stream";
import type { View } from "./stream";
import { WithoutAPage } from "./without-a-page";

export { apiServer } from "./server";

const UNKNOWN_PAGE = "/a/page/that/does/not/exist";
const IDENTITY = "__Host-player";

type Connection = {
  readonly baseURL: string;
  readonly browser: Browser;
  readonly request: APIRequest;
  readonly network: string;
};

/** A Player's HTTP client with its own identity cookie and streams, from its own network. */
class ApiPlayer extends WithoutAPage {
  private readonly view: View = { price: null, delayed: true, score: 0, openGuess: null, lastResolved: null };
  private streams = new AbortController();
  private reading: Promise<void>[] = [];
  private snapshots = 0;
  private lastPage: APIResponse | undefined;
  private identity: string | undefined;
  private refusal: ErrorCode | undefined;
  private turnedAway: { readonly status: number; readonly error: ErrorCode } | undefined;

  constructor(
    private readonly baseURL: string,
    private readonly network: string,
    private readonly request: APIRequestContext,
  ) {
    super();
  }

  async opensAnUnknownPage(): Promise<void> {
    this.lastPage = await this.request.get(UNKNOWN_PAGE);
  }

  async opensThePage(): Promise<void> {
    this.lastPage = await this.request.get("/", { headers: this.headers() });
    this.keepIdentity(this.lastPage.headersArray().filter((header) => header.name.toLowerCase() === "set-cookie"));

    const response = await this.request.get("/api/snapshot", { headers: this.headers() });

    if (response.status() !== 200) {
      this.turnedAway = { status: response.status(), error: (await decode<ErrorBody>(response)).error };
    }
  }

  async opensTheGame(): Promise<void> {
    const opened = this.snapshots;

    const response = await fetch(new URL("/api/events", this.baseURL), {
      headers: this.headers(),
      signal: this.streams.signal,
    });

    expect(response.status, "the stream should open").toBe(200);

    this.identity =
      response.headers.getSetCookie().find((candidate) => candidate.startsWith(`${IDENTITY}=`)) ?? this.identity;

    if (response.body !== null) {
      this.reading.push(this.follow(response.body));
    }

    await expect.poll(() => this.snapshots, "the stream should open with a snapshot").toBeGreaterThan(opened);
  }

  /** Ends every stream the Player holds open. */
  async closesTheGame(): Promise<void> {
    this.streams.abort();
    await Promise.all(this.reading);
    this.streams = new AbortController();
    this.reading = [];
    this.snapshots = 0;
  }

  async guesses(direction: "Up" | "Down"): Promise<void> {
    const command: PlaceGuess = { direction: wire(direction) };

    await this.place(JSON.stringify(command), "application/json");
  }

  async guessesWithAPriceAndTimeOfTheirOwn(direction: "Up" | "Down"): Promise<void> {
    const forged = {
      direction: wire(direction),
      price: 100,
      startPrice: 100,
      time: "2020-01-01T00:00:00Z",
      placedAt: "2020-01-01T00:00:00Z",
      remainingMs: 1,
    };

    await this.place(JSON.stringify(forged), "application/json");
  }

  async guessesADirectionThatIsNeitherUpNorDown(): Promise<void> {
    await this.place(JSON.stringify({ direction: "sideways" }), "application/json");
  }

  async guessesWithABodyThatIsNotJson(): Promise<void> {
    await this.place("direction=up", "application/json");
  }

  async guessesWithoutSayingTheBodyIsJson(): Promise<void> {
    const command: PlaceGuess = { direction: "up" };

    await this.place(JSON.stringify(command), "text/plain");
  }

  async expectsTheGame(): Promise<void> {
    expect(this.lastPage?.status(), "the page should load").toBe(200);
    expect(this.lastPage?.headers()["content-type"]).toContain("text/html");
    expect(await this.lastPage?.text()).toContain("<title>Hoof &amp; Claw</title>");
  }

  async expectsPrice(price: string | null): Promise<void> {
    await expect.poll(() => this.view.price).toBe(price === null ? null : cents(price));
  }

  async expectsPriceDelayed(delayed: boolean): Promise<void> {
    await expect.poll(() => this.view.delayed).toBe(delayed);
  }

  async expectsAnyPrice(): Promise<void> {
    await expect.poll(() => this.view.price, "the stream should show a price").not.toBeNull();
  }

  /** Waits until the stream ends or `milliseconds` pass, and answers how long it stayed open after its snapshot. */
  async holdsTheStream(milliseconds: number): Promise<number> {
    await this.streamOpened();

    return heldOpen(this.reading.at(-1) ?? Promise.resolve(), milliseconds);
  }

  async expectsScore(score: number): Promise<void> {
    await this.streamOpened();
    await expect.poll(() => this.view.score, `the Player should have Score ${String(score)}`).toBe(score);
  }

  async expectsNoOpenGuess(): Promise<void> {
    await this.streamOpened();
    expect(this.view.openGuess).toBeNull();
  }

  async expectsOpenGuess(direction: "Up" | "Down", startPrice: string): Promise<void> {
    await expect
      .poll(
        () =>
          this.view.openGuess && { direction: this.view.openGuess.direction, start: this.view.openGuess.startPrice },
      )
      .toEqual({ direction: wire(direction), start: cents(startPrice) });
  }

  /** The server's clock stands still, so the time left is exactly what the server says. */
  async expectsSecondsLeft(seconds: number): Promise<void> {
    await expect.poll(() => this.view.openGuess?.remainingMs).toBe(seconds * 1000);
  }

  /** The result shows while the Player holds no Open guess, as the page shows it. */
  async expectsResult(shown: "Correct" | "Wrong", startPrice: string, endPrice: string): Promise<void> {
    await expect
      .poll(() => shownResult(this.view), `the Guess should have resolved ${shown} from ${startPrice} to ${endPrice}`)
      .toEqual({ outcome: outcome(shown), start: cents(startPrice), end: cents(endPrice) });
  }

  async expectsNoResult(): Promise<void> {
    await this.streamOpened();
    expect(
      this.view.openGuess !== null || this.view.lastResolved === null,
      "no result of a Resolved guess should show",
    ).toBe(true);
  }

  /** The API has no buttons; a Guess can be placed while the stream shows a live price and no Open guess. */
  async expectsGuessing(possible: boolean): Promise<void> {
    await expect
      .poll(() => !this.view.delayed && this.view.price !== null && this.view.openGuess === null)
      .toBe(possible);
  }

  async expectsNoSecondGuess(): Promise<void> {
    const open = this.view.openGuess;

    await this.guesses("Down");
    expect(this.refusal, "a second Guess should be refused").toBe("openGuessExists");
    expect(this.view.openGuess, "the first Guess should stay open").toEqual(open);
  }

  expectsRefusal(refusal: ErrorCode): Promise<void> {
    expect(this.refusal, `the Guess should be refused as ${refusal}`).toBe(refusal);

    return Promise.resolve();
  }

  async expectsAnIdentity(): Promise<void> {
    await this.streamOpened();
    expect(this.identity, "the server should issue an identity cookie").toBeDefined();

    const attributes = (this.identity ?? "").split(";").map((attribute) => attribute.trim());

    expect(attributes).toEqual(expect.arrayContaining(["Path=/", "HttpOnly", "Secure", "SameSite=Lax"]));
    expect(attributes.find((attribute) => attribute.startsWith("Max-Age="))).toBe("Max-Age=34560000");
  }

  expectsNoIdentity(): Promise<void> {
    expect(this.lastPage?.status(), "the page should load").toBe(200);
    expect(this.identity, "the server should issue no identity cookie").toBeUndefined();

    return Promise.resolve();
  }

  expectsTurnedAway(refusal: ErrorCode): Promise<void> {
    expect(this.turnedAway, `the game should turn the visitor away as ${refusal}`).toEqual({
      status: 429,
      error: refusal,
    });

    return Promise.resolve();
  }

  async close(): Promise<void> {
    this.streams.abort();
    await Promise.all(this.reading);
    await this.request.dispose();
  }

  /** The Player's network and, once issued, their identity cookie. */
  private headers(): Record<string, string> {
    const cookie = this.identity?.split(";")[0];

    return cookie === undefined ? { [FORWARDED_FOR]: this.network } : { [FORWARDED_FOR]: this.network, cookie };
  }

  private keepIdentity(headers: readonly { readonly value: string }[]): void {
    this.identity =
      headers.map((header) => header.value).find((value) => value.startsWith(`${IDENTITY}=`)) ?? this.identity;
  }

  /** Sends the command to place a Guess, as `fetch` does in the page; a `201` answer is the Open guess. */
  private async place(body: string, contentType: string): Promise<void> {
    const response = await fetch(new URL("/api/guesses", this.baseURL), {
      method: "POST",
      headers: { ...this.headers(), "Content-Type": contentType },
      body,
    });

    this.identity =
      response.headers.getSetCookie().find((candidate) => candidate.startsWith(`${IDENTITY}=`)) ?? this.identity;

    if (response.status === 201) {
      this.refusal = undefined;
      this.view.openGuess = await decode<OpenGuess>(response);
    } else {
      this.refusal = (await decode<ErrorBody>(response)).error;
    }
  }

  private async streamOpened(): Promise<void> {
    await expect.poll(() => this.snapshots, "the stream should open with a snapshot").toBeGreaterThan(0);
  }

  private async follow(body: ReadableStream<Uint8Array>): Promise<void> {
    try {
      await read(body, (event) => {
        this.snapshots += event.type === "snapshot" ? 1 : 0;
        apply(this.view, event);
      });
    } catch (error) {
      if (!this.streams.signal.aborted) {
        throw error;
      }
    }
  }
}

export async function apiPlayer(connection: Connection) {
  return new ApiPlayer(
    connection.baseURL,
    connection.network,
    await connection.request.newContext({
      baseURL: connection.baseURL,
      extraHTTPHeaders: { [FORWARDED_FOR]: connection.network },
    }),
  );
}
