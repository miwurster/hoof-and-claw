import { render, screen } from "@testing-library/react";
import { expect, test } from "vitest";

import { OutcomeAnnouncement } from "./OutcomeAnnouncement";

test("the Outcome is announced politely", () => {
  render(<OutcomeAnnouncement outcome="correct" />);

  const announcement = screen.getByRole("status");

  expect(announcement).toHaveAttribute("aria-live", "polite");
  expect(announcement).toHaveTextContent("Your Guess resolved Correct.");
});

test("before an Outcome, the announcement stays empty", () => {
  render(<OutcomeAnnouncement outcome={null} />);

  expect(screen.getByRole("status")).toBeEmptyDOMElement();
});
