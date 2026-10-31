import { expect, test } from "vitest";

import { formatDifference, formatPrice, formatScore, formatScoreChange, formatSeconds } from "./format";

test("a price is formatted in US dollars with cents", () => {
  expect(formatPrice(6_741_218)).toBe("$67,412.18");
  expect(formatPrice(6_400_000)).toBe("$64,000.00");
  expect(formatPrice(5)).toBe("$0.05");
});

test("no price yet is a dash", () => {
  expect(formatPrice(null)).toBe("—");
});

test("a Score change has its sign, a true minus sign when it falls", () => {
  expect(formatScoreChange(1)).toBe("Score +1");
  expect(formatScoreChange(-1)).toBe("Score −1");
});

test("a negative Score has a true minus sign", () => {
  expect(formatScore(-2)).toBe("Score −2");
  expect(formatScore(0)).toBe("Score 0");
  expect(formatScore(3)).toBe("Score 3");
});

test("a difference from the Start price has its sign, dollars with cents and an arrow", () => {
  expect(formatDifference(1_240)).toBe("+$12.40 ↑");
  expect(formatDifference(-310)).toBe("−$3.10 ↓");
  expect(formatDifference(123_456)).toBe("+$1,234.56 ↑");
});

test("no difference from the Start price has no sign and no arrow", () => {
  expect(formatDifference(0)).toBe("$0.00");
});

test("the time left is in whole seconds, rounded up", () => {
  expect(formatSeconds(60_000)).toBe("60");
  expect(formatSeconds(39_001)).toBe("40");
  expect(formatSeconds(1)).toBe("1");
});
