import { render, screen } from "@testing-library/react";
import { expect, test } from "vitest";

import { GuessMessage } from "./GuessMessage";

test("a Guess refused as rateLimited tells the Player to try again in a minute", () => {
  render(<GuessMessage message="rateLimited" />);

  expect(screen.getByText("Too many guesses from your network. Try again in a minute.")).toBeVisible();
});

test("a Guess that failed tells the Player to try again", () => {
  render(<GuessMessage message="failed" />);

  expect(screen.getByText("Couldn't place your guess. Try again.")).toBeVisible();
});

test("without a message there is no line", () => {
  const { container } = render(<GuessMessage message={null} />);

  expect(container).toBeEmptyDOMElement();
});
