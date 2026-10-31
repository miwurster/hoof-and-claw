import { render, screen } from "@testing-library/react";
import { expect, test } from "vitest";

import { ResultCard } from "./ResultCard";
import styles from "./ResultCard.module.css";

/** A class of the card's stylesheet, which must exist for a test about it to mean anything. */
function cssClass(name: string | undefined): string {
  if (name === undefined) {
    throw new Error("the result card's stylesheet should have this class");
  }

  return name;
}

function card() {
  return screen.getByRole("region", { name: "Resolved guess" });
}

test("a Resolved guess shows its Start price, End price, Outcome and the Score change", () => {
  render(
    <ResultCard direction="up" startPrice={6_400_000} endPrice={6_401_240} outcome="correct" reducedMotion={false} />,
  );

  expect(card()).toHaveTextContent("Up");
  expect(card()).toHaveTextContent("Correct");
  expect(card()).toHaveTextContent("Start price $64,000.00");
  expect(card()).toHaveTextContent("End price $64,012.40");
  expect(card()).toHaveTextContent("Score +1");
});

test("a Wrong Outcome takes one off the Score, with a true minus sign", () => {
  render(
    <ResultCard direction="up" startPrice={6_400_000} endPrice={6_399_000} outcome="wrong" reducedMotion={false} />,
  );

  expect(card()).toHaveTextContent("Wrong");
  expect(card()).toHaveTextContent("Score −1");
});

test("the result card stamps a hoof print for a price that went up", () => {
  render(
    <ResultCard direction="down" startPrice={6_400_000} endPrice={6_400_001} outcome="wrong" reducedMotion={false} />,
  );

  expect(screen.getByRole("img", { name: "Hoof print: the price went up" })).toBeVisible();
  expect(screen.queryByRole("img", { name: /Claw mark/u })).toBeNull();
});

test("the result card stamps a claw mark for a price that went down", () => {
  render(
    <ResultCard direction="down" startPrice={6_400_000} endPrice={6_399_999} outcome="correct" reducedMotion={false} />,
  );

  expect(screen.getByRole("img", { name: "Claw mark: the price went down" })).toBeVisible();
  expect(screen.queryByRole("img", { name: /Hoof print/u })).toBeNull();
});

test("the stamp lands and the Score change ticks in time with it", () => {
  render(
    <ResultCard direction="up" startPrice={6_400_000} endPrice={6_401_000} outcome="correct" reducedMotion={false} />,
  );

  expect(screen.getByRole("img", { name: /Hoof print/u })).toHaveClass(cssClass(styles.landing));
  expect(screen.getByText("Score +1")).toHaveClass(cssClass(styles.ticking));
});

test("with prefers-reduced-motion, the stamp and the Score change appear without animation", () => {
  render(<ResultCard direction="up" startPrice={6_400_000} endPrice={6_401_000} outcome="correct" reducedMotion />);

  expect(screen.getByRole("img", { name: /Hoof print/u })).not.toHaveClass(cssClass(styles.landing));
  expect(screen.getByText("Score +1")).not.toHaveClass(cssClass(styles.ticking));
});
