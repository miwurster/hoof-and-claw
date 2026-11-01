// The API driver's translation between glossary terms and the contract's values: prices, Directions, Outcomes, a
// forged Guess, and the result a page shows.
import type { Direction, Outcome } from "contract";

import type { View } from "./stream";

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

/** A command to place a Guess that also carries a price and a time of the Player's choosing. */
export function forgedGuess(direction: "Up" | "Down") {
  return {
    direction: wire(direction),
    price: 100,
    startPrice: 100,
    time: "2020-01-01T00:00:00Z",
    placedAt: "2020-01-01T00:00:00Z",
    remainingMs: 1,
  };
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
