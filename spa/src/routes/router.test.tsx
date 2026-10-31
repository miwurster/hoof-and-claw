import { createMemoryHistory, RouterProvider } from "@tanstack/react-router";
import { render, screen } from "@testing-library/react";
import { expect, test } from "vitest";

import { createGameRouter } from "./router";

test("an unknown path shows the game", async () => {
  const router = createGameRouter(createMemoryHistory({ initialEntries: ["/a/page/that/does/not/exist"] }));

  render(<RouterProvider router={router} />);

  expect(await screen.findByRole("heading", { level: 1, name: "Hoof & Claw" })).toBeVisible();
});
