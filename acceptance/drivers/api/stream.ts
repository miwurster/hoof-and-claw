// What the API driver shares: decoding the contract's bodies and events, and reading the stream.
import type { APIResponse } from "@playwright/test";
import type { Direction, OpenGuess, Outcome, ResolvedGuess, ServerEvent } from "contract";

export const FORWARDED_FOR = "X-Forwarded-For";

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

/** A dollar string such as "64,000.00" in cents. */
export function cents(price: string): number {
  const match = /^(\d{1,3}(?:,\d{3})*|\d+)\.(\d{2})$/u.exec(price);

  if (match?.[1] === undefined || match[2] === undefined) {
    throw new Error(`"${price}" is no price in dollars with cents, such as "64,000.00"`);
  }

  return Number(match[1].replaceAll(",", "")) * 100 + Number(match[2]);
}

export function wire(direction: "Up" | "Down"): Direction {
  return direction === "Up" ? "up" : "down";
}

export function outcome(shown: "Correct" | "Wrong"): Outcome {
  return shown === "Correct" ? "correct" : "wrong";
}

/** The result the page would show: the most recent Resolved guess while there is no Open guess. */
export function shownResult(view: View): { outcome: Outcome; start: number; end: number } | null {
  return view.openGuess === null && view.lastResolved !== null
    ? { outcome: view.lastResolved.outcome, start: view.lastResolved.startPrice, end: view.lastResolved.endPrice }
    : null;
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

/** Reads the stream's events, one `data:` line per event, until it ends or is aborted. */
export async function read(body: ReadableStream<Uint8Array>, onEvent: (event: ServerEvent) => void): Promise<void> {
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
