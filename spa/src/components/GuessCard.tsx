import type { Direction, ResolvedGuess } from "contract";

import { OpenGuessCard } from "./OpenGuessCard";
import { OutcomeAnnouncement } from "./OutcomeAnnouncement";
import { ResultCard } from "./ResultCard";

/** The Open guess as the page counts it down. */
export type CountedGuess = {
  readonly direction: Direction;
  readonly startPrice: number;
  readonly remainingMs: number;
};

/**
 * The guess card: the Open guess while there is one, otherwise the most recent Resolved guess until the next Guess,
 * and nothing on a first visit. Only the Outcome of the Resolved guess is announced.
 */
export function GuessCard({
  open,
  price,
  lastResolved,
  reducedMotion,
}: {
  readonly open: CountedGuess | null;
  readonly price: number | null;
  readonly lastResolved: ResolvedGuess | null;
  readonly reducedMotion: boolean;
}) {
  const result = open === null ? lastResolved : null;

  return (
    <>
      {open === null ? null : (
        <OpenGuessCard
          direction={open.direction}
          startPrice={open.startPrice}
          price={price}
          remainingMs={open.remainingMs}
        />
      )}
      {result === null ? null : (
        <ResultCard
          direction={result.direction}
          startPrice={result.startPrice}
          endPrice={result.endPrice}
          outcome={result.outcome}
          reducedMotion={reducedMotion}
        />
      )}
      <OutcomeAnnouncement outcome={result?.outcome ?? null} />
    </>
  );
}
