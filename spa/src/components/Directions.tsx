import styles from "./Directions.module.css";

/** The Up and Down buttons, enabled only while a Guess can be placed. */
export function Directions({ enabled }: { readonly enabled: boolean }) {
  return (
    <div className={styles.directions}>
      <button className={styles.up} type="button" disabled={!enabled}>
        Up <span aria-hidden="true">↑</span>
      </button>
      <button className={styles.down} type="button" disabled={!enabled}>
        Down <span aria-hidden="true">↓</span>
      </button>
    </div>
  );
}
