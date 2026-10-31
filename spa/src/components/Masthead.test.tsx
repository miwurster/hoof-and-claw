import { render, screen } from "@testing-library/react";
import { expect, test } from "vitest";

import { Masthead } from "./Masthead";

test("the masthead names the game with its logo", () => {
  render(<Masthead score={0} />);

  expect(screen.getByRole("heading", { level: 1, name: "Hoof & Claw" })).toBeVisible();
});
