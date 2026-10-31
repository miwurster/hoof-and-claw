import styles from "./GuessMessage.module.css";

const TEXT = {
  rateLimited: "Too many guesses from your network. Try again in a minute.",
  failed: "Couldn't place your guess. Try again.",
} as const;

/** The one muted line under Up and Down when a Guess could not be placed. */
export function GuessMessage({ message }: { readonly message: keyof typeof TEXT | null }) {
  return message === null ? null : <output className={styles.message}>{TEXT[message]}</output>;
}
