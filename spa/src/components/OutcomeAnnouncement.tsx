import type { Outcome } from "contract";

import styles from "./OutcomeAnnouncement.module.css";

const OUTCOME = { correct: "Correct", wrong: "Wrong" } as const;

/**
 * Announces the Outcome of the Player's Resolved guess politely to a screen reader, and shows nothing.
 * It stays on the page, so a screen reader hears the Outcome when it arrives; the running price is never in it.
 */
export function OutcomeAnnouncement({ outcome }: { readonly outcome: Outcome | null }) {
  return (
    <output aria-live="polite" className={styles.hidden}>
      {outcome === null ? null : `Your Guess resolved ${OUTCOME[outcome]}.`}
    </output>
  );
}
