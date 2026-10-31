import type { ServerEvent } from "contract";
import { expect, test } from "vitest";

import { canGuess, initialState, reduce } from "./reducer";

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

test("before the stream opens, there is no price, the price is delayed and the Score is 0", () => {
  expect(initialState).toEqual({ price: null, delayed: true, score: 0 });
  expect(canGuess(initialState)).toBe(false);
});

test("a snapshot sets the whole state", () => {
  const state = reduce(initialState, { ...snapshot, score: -2 });

  expect(state).toEqual({ price: 6_400_000, delayed: false, score: -2 });
  expect(canGuess(state)).toBe(true);
});

test("a price event changes the price and keeps the Score", () => {
  const state = reduce(reduce(initialState, { ...snapshot, score: 3 }), {
    type: "price",
    price: 6_400_001,
    delayed: false,
  });

  expect(state).toEqual({ price: 6_400_001, delayed: false, score: 3 });
});

test("while the price is delayed, no Guess can be placed", () => {
  const state = reduce(reduce(initialState, snapshot), { type: "price", price: 6_400_000, delayed: true });

  expect(state.price).toBe(6_400_000);
  expect(canGuess(state)).toBe(false);
});

test("a live server without any price yet allows no Guess", () => {
  const state = reduce(initialState, { ...snapshot, price: null });

  expect(canGuess(state)).toBe(false);
});
