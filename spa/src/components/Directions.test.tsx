import { render, screen } from "@testing-library/react";
import { expect, test } from "vitest";

import { Directions } from "./Directions";

test("Up and Down are enabled while a Guess can be placed", () => {
  render(<Directions enabled />);

  expect(screen.getByRole("button", { name: "Up" })).toBeEnabled();
  expect(screen.getByRole("button", { name: "Down" })).toBeEnabled();
});

test("Up and Down are disabled while no Guess can be placed", () => {
  render(<Directions enabled={false} />);

  expect(screen.getByRole("button", { name: "Up" })).toBeDisabled();
  expect(screen.getByRole("button", { name: "Down" })).toBeDisabled();
});
