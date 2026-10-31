import { fireEvent, render, screen } from "@testing-library/react";
import type { Direction } from "contract";
import { expect, test } from "vitest";

import { Directions } from "./Directions";

function ignore(): void {
  // A test that only looks at the buttons does not press them.
}

test("Up and Down are enabled while a Guess can be placed", () => {
  render(<Directions enabled onGuess={ignore} />);

  expect(screen.getByRole("button", { name: "Up" })).toBeEnabled();
  expect(screen.getByRole("button", { name: "Down" })).toBeEnabled();
});

test("Up and Down are disabled while no Guess can be placed", () => {
  render(<Directions enabled={false} onGuess={ignore} />);

  expect(screen.getByRole("button", { name: "Up" })).toBeDisabled();
  expect(screen.getByRole("button", { name: "Down" })).toBeDisabled();
});

test("pressing Up or Down guesses that Direction", () => {
  const guesses: Direction[] = [];

  render(
    <Directions
      enabled
      onGuess={(direction) => {
        guesses.push(direction);
      }}
    />,
  );
  fireEvent.click(screen.getByRole("button", { name: "Up" }));
  fireEvent.click(screen.getByRole("button", { name: "Down" }));

  expect(guesses).toEqual(["up", "down"]);
});
