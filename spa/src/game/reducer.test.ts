import type { OpenGuess, ResolvedGuess, ServerEvent } from "contract";
import { expect, test } from "vitest";

import { canGuess, initialState, reduce } from "./reducer";
import type { Action, GameState } from "./reducer";

const snapshot: ServerEvent = {
  type: "snapshot",
  build: "abc",
  serverTime: "2027-01-01T00:00:00Z",
  price: 6_400_000,
  delayed: false,
  score: 0,
  openGuess: null,
  lastResolved: null,
};

const guess: OpenGuess = { id: "guess-1", direction: "up", startPrice: 6_400_000, remainingMs: 60_000 };

const resolved: ResolvedGuess = {
  id: "guess-1",
  direction: "up",
  startPrice: 6_400_000,
  endPrice: 6_401_000,
  outcome: "correct",
};

function received(event: ServerEvent, at = 0): Action {
  return { type: "received", event, at };
}

function run(...actions: Action[]): GameState {
  return actions.reduce((state, action) => reduce(state, action), initialState);
}

test("before the stream opens, there is no price, the price is delayed and the Score is 0", () => {
  expect(initialState).toEqual({
    price: null,
    delayed: true,
    score: 0,
    openGuess: null,
    lastResolved: null,
    placing: false,
    message: null,
  });
  expect(canGuess(initialState)).toBe(false);
});

test("a snapshot sets the whole state", () => {
  const state = run(received({ ...snapshot, score: -2 }));

  expect(state).toEqual({ ...initialState, price: 6_400_000, delayed: false, score: -2 });
  expect(canGuess(state)).toBe(true);
});

test("a price event changes the price and keeps the Score", () => {
  const state = run(received({ ...snapshot, score: 3 }), received({ type: "price", price: 6_400_001, delayed: false }));

  expect(state).toEqual({ ...initialState, price: 6_400_001, delayed: false, score: 3 });
});

test("while the price is delayed, no Guess can be placed", () => {
  const state = run(received(snapshot), received({ type: "price", price: 6_400_000, delayed: true }));

  expect(state.price).toBe(6_400_000);
  expect(canGuess(state)).toBe(false);
});

test("a live server without any price yet allows no Guess", () => {
  expect(canGuess(run(received({ ...snapshot, price: null })))).toBe(false);
});

test("while a Guess is being placed, no other can be placed", () => {
  const state = run(received(snapshot), { type: "guessing" });

  expect(state.placing).toBe(true);
  expect(canGuess(state)).toBe(false);
});

test("the answer to a Guess makes it the Open guess, falling due its remaining time after it arrived", () => {
  const state = run(received(snapshot), { type: "guessing" }, { type: "guessAnswered", guess, at: 1_000 });

  expect(state.placing).toBe(false);
  expect(state.openGuess).toEqual({ guess, dueAt: 61_000 });
  expect(canGuess(state)).toBe(false);
});

test("the Guess placed event makes it the Open guess too, before or after the answer", () => {
  const placed: ServerEvent = { type: "guessPlaced", guess };

  const eventFirst = run(received(snapshot), { type: "guessing" }, received(placed, 1_000), {
    type: "guessAnswered",
    guess,
    at: 1_200,
  });

  const answerFirst = run(
    received(snapshot),
    { type: "guessing" },
    { type: "guessAnswered", guess, at: 1_000 },
    received(placed, 1_200),
  );

  for (const state of [eventFirst, answerFirst]) {
    expect(state.placing).toBe(false);
    expect(state.openGuess).toEqual({ guess, dueAt: 61_000 });
  }
});

test("a Guess placed in another tab is the Open guess here too", () => {
  const state = run(received(snapshot), received({ type: "guessPlaced", guess }, 500));

  expect(state.openGuess).toEqual({ guess, dueAt: 60_500 });
  expect(canGuess(state)).toBe(false);
});

test("a snapshot's Open guess counts down from when the snapshot arrived", () => {
  const state = run(received({ ...snapshot, openGuess: { ...guess, remainingMs: 40_000 } }, 2_000));

  expect(state.openGuess).toEqual({ guess: { ...guess, remainingMs: 40_000 }, dueAt: 42_000 });
});

test("a refusal as rateLimited tells the Player to try again in a minute", () => {
  const state = run(received(snapshot), { type: "guessing" }, { type: "guessFailed", error: "rateLimited" });

  expect(state.message).toBe("rateLimited");
  expect(state.placing).toBe(false);
  expect(canGuess(state)).toBe(true);
});

test("a Guess that fails with internal or a network error tells the Player to try again", () => {
  for (const error of ["internal", "network"] as const) {
    const state = run(received(snapshot), { type: "guessing" }, { type: "guessFailed", error });

    expect(state.message, error).toBe("failed");
    expect(state.placing, error).toBe(false);
  }
});

test("a refusal as openGuessExists or priceDelayed shows no message, because the stream already shows the state", () => {
  for (const error of ["openGuessExists", "priceDelayed"] as const) {
    const state = run(received(snapshot), { type: "guessing" }, { type: "guessFailed", error });

    expect(state.message, error).toBeNull();
    expect(state.placing, error).toBe(false);
  }
});

test("pressing again clears the last message", () => {
  const state = run(
    received(snapshot),
    { type: "guessing" },
    { type: "guessFailed", error: "internal" },
    { type: "guessing" },
  );

  expect(state.message).toBeNull();
});

test("a snapshot carries the most recent Resolved guess, for a Player who was away when it resolved", () => {
  const state = run(received({ ...snapshot, score: 1, lastResolved: resolved }));

  expect(state.lastResolved).toEqual(resolved);
  expect(state.score).toBe(1);
});

test("the Open guess resolving ends it, shows its result and sets the new Score", () => {
  const state = run(received(snapshot), received({ type: "guessPlaced", guess }), {
    type: "received",
    event: { type: "guessResolved", guess: resolved, score: 1 },
    at: 61_000,
  });

  expect(state.openGuess).toBeNull();
  expect(state.lastResolved).toEqual(resolved);
  expect(state.score).toBe(1);
  expect(canGuess(state)).toBe(true);
});

test("a resolution for a Guess the page does not hold as open changes nothing", () => {
  const before = run(received({ ...snapshot, score: 1, lastResolved: resolved }));

  const after = reduce(before, received({ type: "guessResolved", guess: resolved, score: 1 }));

  expect(after).toEqual(before);
});

test("the result stays when the next Guess is placed, beneath it", () => {
  const next: OpenGuess = { ...guess, id: "guess-2" };

  const state = run(received({ ...snapshot, lastResolved: resolved }), received({ type: "guessPlaced", guess: next }));

  expect(state.openGuess?.guess).toEqual(next);
  expect(state.lastResolved).toEqual(resolved);
});

test("an answer that arrives after its Guess resolved does not open it again", () => {
  const state = run(
    received(snapshot),
    { type: "guessing" },
    received({ type: "guessPlaced", guess }),
    received({ type: "guessResolved", guess: resolved, score: 1 }),
    { type: "guessAnswered", guess, at: 1_000 },
  );

  expect(state.openGuess).toBeNull();
  expect(state.lastResolved).toEqual(resolved);
  expect(state.placing).toBe(false);
});

test("a lost stream keeps the last price, the Score and the Open guess, delayed, and allows no Guess", () => {
  const state = run(received({ ...snapshot, score: 2 }), received({ type: "guessPlaced", guess }), {
    type: "streamLost",
  });

  expect(state.price).toBe(6_400_000);
  expect(state.delayed).toBe(true);
  expect(state.score).toBe(2);
  expect(state.openGuess).toEqual({ guess, dueAt: 60_000 });
  expect(canGuess(state)).toBe(false);
});
