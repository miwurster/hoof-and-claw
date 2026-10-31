import type { ResolvedGuess } from "contract";
import { render, screen } from "@testing-library/react";
import { expect, test } from "vitest";

import { GuessCard } from "./GuessCard";

const resolved: ResolvedGuess = {
  id: "guess-1",
  direction: "up",
  startPrice: 6_400_000,
  endPrice: 6_401_000,
  outcome: "correct",
};

const open = { direction: "down", startPrice: 6_401_000, remainingMs: 30_000 } as const;

test("a first visit with no Guess shows no guess card", () => {
  render(<GuessCard open={null} price={6_400_000} lastResolved={null} reducedMotion={false} />);

  expect(screen.queryByRole("region", { name: "Open guess" })).toBeNull();
  expect(screen.queryByRole("region", { name: "Resolved guess" })).toBeNull();
});

test("a Resolved guess shows its result and announces its Outcome", () => {
  render(<GuessCard open={null} price={6_401_000} lastResolved={resolved} reducedMotion={false} />);

  expect(screen.getByRole("region", { name: "Resolved guess" })).toBeVisible();
  expect(screen.getByRole("status")).toHaveTextContent("Your Guess resolved Correct.");
});

test("the next Guess takes the place of the result", () => {
  render(<GuessCard open={open} price={6_401_000} lastResolved={resolved} reducedMotion={false} />);

  expect(screen.getByRole("region", { name: "Open guess" })).toBeVisible();
  expect(screen.queryByRole("region", { name: "Resolved guess" })).toBeNull();
  expect(screen.getByRole("status")).toBeEmptyDOMElement();
});
