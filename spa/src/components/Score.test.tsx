import { render, screen } from "@testing-library/react";
import { expect, test } from "vitest";

import { Score } from "./Score";

test("the Score is reachable by its name", () => {
  render(<Score score={3} />);

  expect(screen.getByRole("region", { name: "Score" })).toHaveTextContent("Score 3");
});

test("a negative Score shows a true minus sign", () => {
  render(<Score score={-2} />);

  expect(screen.getByRole("region", { name: "Score" })).toHaveTextContent("Score −2");
});
