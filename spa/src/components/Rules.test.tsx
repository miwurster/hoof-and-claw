import { render, screen } from "@testing-library/react";
import { expect, test } from "vitest";

import { Rules } from "./Rules";

test("the rules line says how a Guess scores", () => {
  render(<Rules />);

  expect(screen.getByText("Guess where BTC goes in the next minute. Right +1, wrong −1.")).toBeVisible();
});

test("the page says where the Score is kept", () => {
  render(<Rules />);

  expect(screen.getByText("Your score lives in this browser.")).toBeVisible();
});
