import type { Direction, Outcome } from "contract";

import { formatPrice, formatScoreChange } from "./format";
import styles from "./ResultCard.module.css";
import clawMark from "./stamps/claw-mark.svg";
import hoofPrint from "./stamps/hoof-print.svg";

const DIRECTION = { up: "Up ↑", down: "Down ↓" } as const;

const OUTCOME = { correct: "Correct", wrong: "Wrong" } as const;

const SCORE_CHANGE = { correct: 1, wrong: -1 } as const;

/**
 * The Resolved guess, in place of the Open guess until the next Guess: its Start price and End price, the Outcome,
 * the Score change, and a stamp for which way the price went. The stamp lands and the Score change ticks in time with
 * it, unless the Player prefers reduced motion.
 */
export function ResultCard({
  direction,
  startPrice,
  endPrice,
  outcome,
  reducedMotion,
}: {
  readonly direction: Direction;
  readonly startPrice: number;
  readonly endPrice: number;
  readonly outcome: Outcome;
  readonly reducedMotion: boolean;
}) {
  const wentUp = endPrice > startPrice;

  return (
    <section className={styles.card} aria-label="Resolved guess">
      <p className={styles[direction]}>{DIRECTION[direction]}</p>
      <img
        className={reducedMotion ? styles.stamp : `${styles.stamp} ${styles.landing}`}
        src={wentUp ? hoofPrint : clawMark}
        alt={wentUp ? "Hoof print: the price went up" : "Claw mark: the price went down"}
      />
      <p className={styles.outcome}>{OUTCOME[outcome]}</p>
      <p className={reducedMotion ? styles.change : `${styles.change} ${styles.ticking}`}>
        {formatScoreChange(SCORE_CHANGE[outcome])}
      </p>
      <p className={styles.prices}>Start price {formatPrice(startPrice)}</p>
      <p className={styles.prices}>End price {formatPrice(endPrice)}</p>
    </section>
  );
}
