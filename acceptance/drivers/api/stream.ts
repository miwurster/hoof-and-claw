// What the API driver shares: decoding the contract's bodies and events, and reading the stream.
import { setTimeout as delay } from "node:timers/promises";

import type { APIResponse } from "@playwright/test";
import type { OpenGuess, ResolvedGuess, ServerEvent } from "contract";

export const FORWARDED_FOR = "X-Forwarded-For";

/** How long the driver waits before it opens a stream again that the server ended, about as long as the page waits. */
const RECONNECT_MS = 250;

/** What the Player's stream and answers have shown so far. */
export type View = {
  price: number | null;
  delayed: boolean;
  score: number;
  openGuess: OpenGuess | null;
  lastResolved: ResolvedGuess | null;
};

/** Decodes a body as the contract's type; the one place this driver trusts the server's JSON. */
export async function decode<Body>(response: APIResponse | Response): Promise<Body> {
  // SAFETY: the server and this driver are built from the same commit, so the body is the contract's type (ADR 0014).
  // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- Playwright and fetch type every JSON body as any.
  return (await response.json()) as Body;
}

/** Decodes one event of the stream. */
function decodeEvent(data: string): ServerEvent {
  // SAFETY: the server and this driver are built from the same commit, so the event is the contract's type (ADR 0014).
  // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- JSON.parse returns any.
  return JSON.parse(data) as ServerEvent;
}

export function apply(view: View, event: ServerEvent): void {
  if (event.type === "guessPlaced") {
    view.openGuess = event.guess;

    return;
  }

  if (event.type === "guessResolved") {
    // As the page does: only the Open guess it holds can resolve, so a repeated event changes nothing (ADR 0010).
    if (view.openGuess?.id === event.guess.id) {
      view.openGuess = null;
      view.lastResolved = event.guess;
      view.score = event.score;
    }

    return;
  }

  view.price = event.price;
  view.delayed = event.delayed;

  if (event.type === "snapshot") {
    view.score = event.score;
    view.openGuess = event.openGuess;
    view.lastResolved = event.lastResolved;
  }
}

/** Hands one event, given as its lines, to the listener its name picks, as `EventSource` does. */
function dispatch(lines: string[], onEvent: (event: ServerEvent) => void, onKeepalive: () => void): void {
  const name = lines
    .find((line) => line.startsWith("event:"))
    ?.slice("event:".length)
    .trim();

  if (name === "keepalive") {
    onKeepalive();
  } else if (name === undefined) {
    for (const line of lines) {
      if (line.startsWith("data:")) {
        onEvent(decodeEvent(line.slice("data:".length).trim()));
      }
    }
  }
}

/**
 * Reads the stream's events, one `data:` line per event, until it ends or is aborted.
 * As a page's `EventSource` does, it hands on only unnamed events; the named `keepalive` goes to `onKeepalive`.
 */
export async function read(
  body: ReadableStream<Uint8Array>,
  onEvent: (event: ServerEvent) => void,
  onKeepalive: () => void = () => {},
): Promise<void> {
  const decoder = new TextDecoder();
  let buffer = "";

  for await (const chunk of body) {
    buffer += decoder.decode(chunk, { stream: true });

    let end = buffer.indexOf("\n\n");

    while (end !== -1) {
      dispatch(buffer.slice(0, end).split("\n"), onEvent, onKeepalive);
      buffer = buffer.slice(end + 2);

      end = buffer.indexOf("\n\n");
    }
  }
}

/**
 * Reads the stream's events, and once the server ends or loses it, opens a new one as the page does, until `signal`
 * aborts; the new stream's snapshot rebuilds the view (ADR 0010).
 */
export async function follow(
  body: ReadableStream<Uint8Array>,
  reopen: (signal: AbortSignal) => Promise<Response>,
  signal: AbortSignal,
  onEvent: (event: ServerEvent) => void,
  onKeepalive: () => void,
): Promise<void> {
  let current: ReadableStream<Uint8Array> | null = body;

  while (!signal.aborted) {
    if (current !== null) {
      try {
        await read(current, onEvent, onKeepalive);
      } catch {
        // A stream the server lost reconnects as one it ended does.
      }
    }

    current = null;

    try {
      await delay(RECONNECT_MS, undefined, { signal });

      const response = await reopen(signal);

      current = response.status === 200 ? response.body : null;
    } catch {
      // The server is not back yet, or the Player closed the game.
    }
  }
}

/** Waits until the stream has `ended` or `milliseconds` pass, and answers how long that took. */
export async function heldOpen(ended: Promise<void>, milliseconds: number): Promise<number> {
  const started = Date.now();
  let timer: ReturnType<typeof setTimeout> | undefined;

  const held = new Promise<void>((resolve) => {
    timer = setTimeout(resolve, milliseconds);
  });

  await Promise.race([ended, held]);
  clearTimeout(timer);

  return Date.now() - started;
}
