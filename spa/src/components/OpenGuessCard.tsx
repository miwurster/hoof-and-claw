import type { Direction } from "contract";

import { formatDifference, formatPrice, formatSeconds } from "./format";
import styles from "./OpenGuessCard.module.css";

const GUESS_MS = 60_000;

/** The ring's circumference in the units of its view box. */
const RING = 2 * Math.PI * 45;

const DIRECTION = { up: "Up ↑", down: "Down ↓" } as const;

/** How the difference from the Start price stands for the Guess right now. */
function standing(direction: Direction, difference: number): "correct" | "wrong" | "even" {
  if (difference === 0) {
    return "even";
  }

  return (direction === "up") === difference > 0 ? "correct" : "wrong";
}

/**
 * The Open guess: a ring that drains over 60 seconds with the seconds left in its centre, the Start price, and the
 * live difference from it, coloured by whether the Guess would currently be Correct.
 */
export function OpenGuessCard({
  direction,
  startPrice,
  price,
  remainingMs,
}: {
  readonly direction: Direction;
  readonly startPrice: number;
  readonly price: number | null;
  readonly remainingMs: number;
}) {
  const difference = (price ?? startPrice) - startPrice;
  const waiting = remainingMs <= 0;
  const drained = RING * (1 - Math.min(remainingMs, GUESS_MS) / GUESS_MS);

  return (
    <section className={styles.card} aria-label="Open guess">
      <p className={styles[direction]}>{DIRECTION[direction]}</p>
      <div className={waiting ? styles.pulse : styles.timer}>
        <svg className={styles.ring} viewBox="0 0 100 100" aria-hidden="true">
          <circle className={styles.track} cx="50" cy="50" r="45" />
          <circle
            className={styles.left}
            cx="50"
            cy="50"
            r="45"
            strokeDasharray={RING}
            strokeDashoffset={waiting ? 0 : drained}
          />
        </svg>
        {waiting ? null : (
          <p className={styles.seconds}>
            {formatSeconds(remainingMs)}
            <span className={styles.hidden}> seconds left</span>
          </p>
        )}
      </div>
      {waiting ? <p className={styles.waiting}>Waiting for the price to move…</p> : null}
      <p className={styles.start}>Start price {formatPrice(startPrice)}</p>
      <p className={styles[standing(direction, difference)]}>{formatDifference(difference)}</p>
    </section>
  );
}
