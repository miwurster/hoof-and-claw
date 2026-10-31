import type { Direction } from "contract";

import styles from "./Directions.module.css";

/** The Up and Down buttons, enabled only while a Guess can be placed. */
export function Directions({
  enabled,
  onGuess,
}: {
  readonly enabled: boolean;
  readonly onGuess: (direction: Direction) => void;
}) {
  return (
    <div className={styles.directions}>
      <button
        className={styles.up}
        type="button"
        disabled={!enabled}
        onClick={() => {
          onGuess("up");
        }}
      >
        Up <span aria-hidden="true">↑</span>
      </button>
      <button
        className={styles.down}
        type="button"
        disabled={!enabled}
        onClick={() => {
          onGuess("down");
        }}
      >
        Down <span aria-hidden="true">↓</span>
      </button>
    </div>
  );
}
