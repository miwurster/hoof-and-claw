import { render, screen } from "@testing-library/react";
import { expect, test } from "vitest";

import { OpenGuessCard } from "./OpenGuessCard";

function card() {
  return screen.getByRole("region", { name: "Open guess" });
}

test("the Open guess shows its Direction, Start price and the seconds left", () => {
  render(<OpenGuessCard direction="up" startPrice={6_400_000} price={6_400_000} remainingMs={40_000} />);

  expect(card()).toHaveTextContent("Up");
  expect(card()).toHaveTextContent("Start price $64,000.00");
  expect(card()).toHaveTextContent("40 seconds left");
});

test("a Guess Down shows Down", () => {
  render(<OpenGuessCard direction="down" startPrice={6_400_000} price={6_400_000} remainingMs={60_000} />);

  expect(card()).toHaveTextContent("Down");
  expect(card()).not.toHaveTextContent("Up");
});

test("the Open guess shows the live difference from the Start price", () => {
  render(<OpenGuessCard direction="up" startPrice={6_400_000} price={6_401_240} remainingMs={30_000} />);

  expect(screen.getByText("+$12.40 ↑")).toBeVisible();
});

test("a price equal to the Start price shows $0.00 with no arrow", () => {
  render(<OpenGuessCard direction="down" startPrice={6_400_000} price={6_400_000} remainingMs={30_000} />);

  expect(screen.getByText("$0.00")).toBeVisible();
});

test("the live difference is coloured by whether the Guess would currently be Correct", () => {
  const { rerender } = render(
    <OpenGuessCard direction="down" startPrice={6_400_000} price={6_399_000} remainingMs={30_000} />,
  );

  const correct = screen.getByText("−$10.00 ↓").className;

  rerender(<OpenGuessCard direction="up" startPrice={6_400_000} price={6_399_000} remainingMs={30_000} />);

  const wrong = screen.getByText("−$10.00 ↓").className;

  rerender(<OpenGuessCard direction="up" startPrice={6_400_000} price={6_400_000} remainingMs={30_000} />);

  const even = screen.getByText("$0.00").className;

  expect(new Set([correct, wrong, even]).size).toBe(3);
});

test("once no time is left, the Open guess waits for the price to move", () => {
  render(<OpenGuessCard direction="up" startPrice={6_400_000} price={6_400_000} remainingMs={0} />);

  expect(card()).toHaveTextContent("Waiting for the price to move…");
  expect(card()).not.toHaveTextContent("seconds left");
});
