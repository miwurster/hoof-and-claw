import styles from "./Rules.module.css";

/** The muted rules line, always present, and where the Score is kept. */
export function Rules() {
  return (
    <div className={styles.rules}>
      <p>Guess where BTC goes in the next minute. Right +1, wrong −1.</p>
      <p>Your score lives in this browser.</p>
    </div>
  );
}
