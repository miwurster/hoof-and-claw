import type { Direction, ErrorBody, OpenGuess, PlaceGuess, ServerEvent } from "contract";
import { useCallback, useEffect, useReducer } from "react";

import { canGuess, initialState, reduce } from "./reducer";
import type { Action, GameState } from "./reducer";

/** The part of `EventSource` the hook uses. */
export type Stream = {
  addEventListener: (type: "message", listener: (message: MessageEvent<string>) => void) => void;
  close: () => void;
};

/** The part of `fetch` the hook uses. */
export type Send = (url: string, init: RequestInit) => Promise<Response>;

/** The game's state, whether a Guess can be placed now, and placing one. */
export type Game = {
  readonly state: GameState;
  readonly canGuess: boolean;
  readonly place: (direction: Direction) => Promise<void>;
};

function openEventSource(url: string): Stream {
  return new EventSource(url);
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

/** The game's state, fed by the server's stream for as long as the page shows the game, and placing a Guess. */
export function useGame(
  openStream: (url: string) => Stream = openEventSource,
  send: Send = sendFetch,
  now: () => number = pageClock,
): Game {
  const [state, dispatch] = useReducer(reduce, initialState);

  useEffect(() => {
    const stream = openStream("/api/events");

    stream.addEventListener("message", (message) => {
      dispatch({ type: "received", event: decode<ServerEvent>(message.data), at: now() });
    });

    return () => {
      stream.close();
    };
  }, [openStream, now]);

  const place = useCallback(
    async (direction: Direction) => {
      dispatch({ type: "guessing" });
      dispatch(await placeGuess(send, now, direction));
    },
    [send, now],
  );

  return { state, canGuess: canGuess(state), place };
}
