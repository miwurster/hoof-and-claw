// The API driver: the game's HTTP API and its stream, as the SPA uses them.
import { expect } from "@playwright/test";
import type { APIRequest, APIRequestContext, APIResponse, Browser } from "@playwright/test";
import type { ErrorBody, Health, OpenGuess, ServerEvent } from "contract";

const UNKNOWN_PAGE = "/a/page/that/does/not/exist";
const MISSING_GAME_ROUTE = "/api/a-route-that-does-not-exist";
const MISSING_TEST_CONTROL = "/test-controls/a-control-that-does-not-exist";
const IDENTITY = "__Host-player";

type Connection = {
  readonly baseURL: string;
  readonly browser: Browser;
  readonly request: APIRequest;
};

/** What the Player's stream has shown so far. */
type View = {
  price: number | null;
  delayed: boolean;
  score: number;
  openGuess: OpenGuess | null;
};

/** Decodes a body as the contract's type; the one place this driver trusts the server's JSON. */
async function decode<Body>(response: APIResponse): Promise<Body> {
  // SAFETY: the server and this driver are built from the same commit, so the body is the contract's type (ADR 0014).
  // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- Playwright types every JSON body as any.
  return (await response.json()) as Body;
}

/** Decodes one event of the stream. */
function decodeEvent(data: string): ServerEvent {
  // SAFETY: the server and this driver are built from the same commit, so the event is the contract's type (ADR 0014).
  // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- JSON.parse returns any.
  return JSON.parse(data) as ServerEvent;
}

/** A dollar string such as "64,000.00" in cents. */
function cents(price: string): number {
  const match = /^(\d{1,3}(?:,\d{3})*|\d+)\.(\d{2})$/u.exec(price);

  if (match?.[1] === undefined || match[2] === undefined) {
    throw new Error(`"${price}" is no price in dollars with cents, such as "64,000.00"`);
  }

  return Number(match[1].replaceAll(",", "")) * 100 + Number(match[2]);
}

function apply(view: View, event: ServerEvent): void {
  view.price = event.price;
  view.delayed = event.delayed;

  if (event.type === "snapshot") {
    view.score = event.score;
    view.openGuess = event.openGuess;
  }
}

/** Reads the stream's events, one `data:` line per event, until it ends or is aborted. */
async function read(body: ReadableStream<Uint8Array>, onEvent: (event: ServerEvent) => void): Promise<void> {
  const decoder = new TextDecoder();
  let buffer = "";

  for await (const chunk of body) {
    buffer += decoder.decode(chunk, { stream: true });

    let end = buffer.indexOf("\n\n");

    while (end !== -1) {
      const lines = buffer.slice(0, end).split("\n");

      buffer = buffer.slice(end + 2);

      for (const line of lines) {
        if (line.startsWith("data:")) {
          onEvent(decodeEvent(line.slice("data:".length).trim()));
        }
      }

      end = buffer.indexOf("\n\n");
    }
  }
}

async function missing(response: APIResponse) {
  const body = await decode<ErrorBody>(response);

  return { status: response.status(), error: body.error };
}

/** A Player's HTTP client with its own identity cookie and streams. */
class ApiPlayer {
  private readonly view: View = { price: null, delayed: true, score: 0, openGuess: null };
  private readonly streams = new AbortController();
  private readonly reading: Promise<void>[] = [];
  private snapshots = 0;
  private lastPage: APIResponse | undefined;
  private identity: string | undefined;

  constructor(
    private readonly baseURL: string,
    private readonly request: APIRequestContext,
  ) {}

  async opensAnUnknownPage(): Promise<void> {
    this.lastPage = await this.request.get(UNKNOWN_PAGE);
  }

  async opensTheGame(): Promise<void> {
    const cookie = this.identity?.split(";")[0];
    const headers: Record<string, string> = cookie === undefined ? {} : { cookie };
    const response = await fetch(new URL("/api/events", this.baseURL), { headers, signal: this.streams.signal });

    expect(response.status, "the stream should open").toBe(200);

    this.identity =
      response.headers.getSetCookie().find((candidate) => candidate.startsWith(`${IDENTITY}=`)) ?? this.identity;

    if (response.body !== null) {
      this.reading.push(this.follow(response.body));
    }

    await this.streamOpened();
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

    const opened = Date.now();
    const ended = this.reading.at(-1) ?? Promise.resolve();
    let timer: ReturnType<typeof setTimeout> | undefined;

    const held = new Promise<void>((resolve) => {
      timer = setTimeout(resolve, milliseconds);
    });

    await Promise.race([ended, held]);
    clearTimeout(timer);

    return Date.now() - opened;
  }

  async expectsScore(score: number): Promise<void> {
    await this.streamOpened();
    expect(this.view.score).toBe(score);
  }

  async expectsNoOpenGuess(): Promise<void> {
    await this.streamOpened();
    expect(this.view.openGuess).toBeNull();
  }

  /** The API has no buttons; a Guess can be placed while the stream shows a live price. */
  async expectsGuessing(possible: boolean): Promise<void> {
    await expect.poll(() => !this.view.delayed && this.view.price !== null).toBe(possible);
  }

  async expectsAnIdentity(): Promise<void> {
    await this.streamOpened();
    expect(this.identity, "the server should issue an identity cookie").toBeDefined();

    const attributes = (this.identity ?? "").split(";").map((attribute) => attribute.trim());

    expect(attributes).toEqual(expect.arrayContaining(["Path=/", "HttpOnly", "Secure", "SameSite=Lax"]));
    expect(attributes.find((attribute) => attribute.startsWith("Max-Age="))).toBe("Max-Age=34560000");
  }

  expectsAnnouncement(): Promise<void> {
    return Promise.reject(new Error("the API has no screen reader"));
  }

  expectsWhereTheScoreLives(): Promise<void> {
    return Promise.reject(new Error("the API has no page"));
  }

  async close(): Promise<void> {
    this.streams.abort();
    await Promise.all(this.reading);
    await this.request.dispose();
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
  return new ApiPlayer(connection.baseURL, await connection.request.newContext({ baseURL: connection.baseURL }));
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
