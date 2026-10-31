import { render, screen } from "@testing-library/react";
import { expect, test } from "vitest";

import { Price } from "./Price";

test("a live price shows in US dollars with no Price delayed label", () => {
  render(<Price price={6_741_218} delayed={false} />);

  expect(screen.getByRole("region", { name: "BTC/USD price" })).toHaveTextContent("$67,412.18");
  expect(screen.queryByText("Price delayed")).toBeNull();
});

test("a delayed price shows the last price labelled Price delayed", () => {
  render(<Price price={6_741_218} delayed />);

  expect(screen.getByRole("region", { name: "BTC/USD price" })).toHaveTextContent("$67,412.18");
  expect(screen.getByText("Price delayed")).toBeVisible();
});

test("before any price, a dash shows labelled Price delayed", () => {
  render(<Price price={null} delayed />);

  expect(screen.getByRole("region", { name: "BTC/USD price" })).toHaveTextContent("—");
  expect(screen.getByText("Price delayed")).toBeVisible();
});

test("a screen reader hears politely whether the price is live or delayed", () => {
  const { rerender } = render(<Price price={6_741_218} delayed />);
  const status = screen.getByRole("status");

  expect(status).toHaveTextContent("Price delayed");

  rerender(<Price price={6_741_218} delayed={false} />);

  expect(screen.getByRole("status")).toBe(status);
  expect(status).toHaveTextContent("Price live");
});

test("the running price is not announced", () => {
  render(<Price price={6_741_218} delayed={false} />);

  expect(screen.getByRole("status")).not.toHaveTextContent("$67,412.18");
});
