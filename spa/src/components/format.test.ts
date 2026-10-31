import { expect, test } from "vitest";

import { formatPrice, formatScore } from "./format";

test("a price is formatted in US dollars with cents", () => {
  expect(formatPrice(6_741_218)).toBe("$67,412.18");
  expect(formatPrice(6_400_000)).toBe("$64,000.00");
  expect(formatPrice(5)).toBe("$0.05");
});

test("no price yet is a dash", () => {
  expect(formatPrice(null)).toBe("—");
});

test("a negative Score has a true minus sign", () => {
  expect(formatScore(-2)).toBe("Score −2");
  expect(formatScore(0)).toBe("Score 0");
  expect(formatScore(3)).toBe("Score 3");
});
