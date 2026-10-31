import type { Direction, ErrorBody, OpenGuess, PlaceGuess, ServerEvent } from "contract";
import { useCallback, useEffect, useReducer } from "react";

import { canGuess, initialState, reduce } from "./reducer";
import type { Action, GameState } from "./reducer";

/** The part of `EventSource` the hook uses. */
export type Stream = {
  addEventListener: (type: "message" | "error", listener: (message: MessageEvent<string>) => void) => void;
  close: () => void;
};

/** What the hook needs from the browser for its stream, so a test can stand in for it. */
export type Browser = {
  readonly openStream: (url: string) => Stream;
  /** A random number in [0, 1), as `Math.random` returns. */
  readonly random: () => number;
  readonly reload: () => void;
};

/** The part of `fetch` the hook uses. */
export type Send = (url: string, init: RequestInit) => Promise<Response>;

/** The game's state, whether a Guess can be placed now, and placing one. */
export type Game = {
  readonly state: GameState;
  readonly canGuess: boolean;
  readonly place: (direction: Direction) => Promise<void>;
};

const theBrowser: Browser = {
  openStream: (url) => new EventSource(url),
  random: () => Math.random(),
  reload: () => {
    globalThis.location.reload();
  },
};

/** A stream without any data for this long is dead (ADR 0010). */
const SILENCE_LIMIT_MS = 45_000;

/** The first reconnect waits up to this long; every further one up to twice as long as the one before. */
const FIRST_DELAY_MS = 1_000;

/** No reconnect waits longer than this. */
const LONGEST_DELAY_MS = 30_000;

/**
 * How long to wait before reconnect number `attempt`, counted from 0 since the last snapshot.
 * Half the delay is fixed and half random, so the delay grows with every attempt and tabs spread out after a deploy.
 */
function reconnectDelay(attempt: number, random: number): number {
  const ceiling = Math.min(LONGEST_DELAY_MS, FIRST_DELAY_MS * 2 ** attempt);

  return (ceiling / 2) * (1 + random);
}

function sendFetch(url: string, init: RequestInit): Promise<Response> {
  return fetch(url, init);
}

/** The page's own clock, which the countdown runs on; never compared with the server's time. */
function pageClock(): number {
  return performance.now();
}

/** Decodes a response body or an event of the stream; the one place the SPA trusts the server's JSON. */
// oxlint-disable-next-line typescript/no-unnecessary-type-parameters -- one decoder for every body is the one place ADR 0014 asks for.
function decode<Body>(text: string): Body {
  // SAFETY: the SPA and the server come from the same commit and the same origin (ADR 0014).
  // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- JSON.parse returns any.
  return JSON.parse(text) as Body;
}

/** The error code of a refused Guess, or `internal` for an answer that is no error body, such as a proxy's page. */
function errorCode(text: string): ErrorBody["error"] {
  try {
    return decode<ErrorBody>(text).error;
  } catch {
    return "internal";
  }
}

/** Sends the command to place a Guess and turns its answer into what happened. */
async function placeGuess(send: Send, now: () => number, direction: Direction): Promise<Action> {
  const command: PlaceGuess = { direction };
  let status: number;
  let text: string;

  try {
    const response = await send("/api/guesses", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(command),
    });

    status = response.status;
    text = await response.text();
  } catch {
    return { type: "guessFailed", error: "network" };
  }

  return status === 201
    ? { type: "guessAnswered", guess: decode<OpenGuess>(text), at: now() }
    : { type: "guessFailed", error: errorCode(text) };
}

/**
 * The page's one stream, which the hook alone reconnects (ADR 0014): on an error, and after 45 s without data, it
 * closes the stream, shows the price as delayed, and opens a new one after a random, growing delay.
 * The new stream's snapshot rebuilds the state.
 */
class Connection {
  private stream: Stream | undefined;
  private silence: ReturnType<typeof setTimeout> | undefined;
  private reconnect: ReturnType<typeof setTimeout> | undefined;
  /** Reconnects since the last snapshot. */
  private attempt = 0;
  /** The build of the first snapshot: the build of the server that served this page. */
  private ownBuild: string | undefined;

  constructor(
    private readonly browser: Browser,
    private readonly now: () => number,
    private readonly dispatch: (action: Action) => void,
  ) {}

  open(): void {
    this.close();

    const opened = this.browser.openStream("/api/events");

    this.stream = opened;
    opened.addEventListener("message", (message) => {
      if (this.stream === opened) {
        this.listen();
        this.receive(decode<ServerEvent>(message.data));
      }
    });
    opened.addEventListener("error", () => {
      if (this.stream === opened) {
        this.lose();
      }
    });
    this.listen();
  }

  close(): void {
    clearTimeout(this.silence);
    clearTimeout(this.reconnect);
    this.stream?.close();
    this.stream = undefined;
  }

  private lose(): void {
    this.close();
    this.dispatch({ type: "streamLost" });
    this.reconnect = setTimeout(
      () => {
        this.open();
      },
      reconnectDelay(this.attempt, this.browser.random()),
    );
    this.attempt += 1;
  }

  /** Starts counting the silence anew. */
  private listen(): void {
    clearTimeout(this.silence);
    this.silence = setTimeout(() => {
      this.lose();
    }, SILENCE_LIMIT_MS);
  }

  private receive(event: ServerEvent): void {
    if (event.type === "snapshot") {
      this.ownBuild ??= event.build;

      if (event.build !== this.ownBuild) {
        this.close();
        this.browser.reload();

        return;
      }

      this.attempt = 0;
    }

    this.dispatch({ type: "received", event, at: this.now() });
  }
}

/** The game's state, fed by the server's stream for as long as the page shows the game, and placing a Guess. */
export function useGame(browser: Browser = theBrowser, send: Send = sendFetch, now: () => number = pageClock): Game {
  const [state, dispatch] = useReducer(reduce, initialState);

  useEffect(() => {
    const connection = new Connection(browser, now, dispatch);

    connection.open();

    return () => {
      connection.close();
    };
  }, [browser, now]);

  const place = useCallback(
    async (direction: Direction) => {
      dispatch({ type: "guessing" });
      dispatch(await placeGuess(send, now, direction));
    },
    [send, now],
  );

  return { state, canGuess: canGuess(state), place };
}
