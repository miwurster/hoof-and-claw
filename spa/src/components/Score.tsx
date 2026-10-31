import { formatScore } from "./format";
import styles from "./Score.module.css";

/** The Player's Score, always visible beside the logo. */
export function Score({ score }: { readonly score: number }) {
  return (
    <section className={styles.score} aria-label="Score">
      {formatScore(score)}
    </section>
  );
}
