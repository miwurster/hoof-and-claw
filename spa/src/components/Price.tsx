import { formatPrice } from "./format";
import styles from "./Price.module.css";

/**
 * The latest BTC/USD price, in Flint and labelled "Price delayed" while it is delayed.
 * Only the change between live and Price delayed is announced, never the running price.
 */
export function Price({ price, delayed }: { readonly price: number | null; readonly delayed: boolean }) {
  return (
    <section className={styles.board} aria-label="BTC/USD price">
      <p className={delayed ? styles.delayedPrice : styles.price}>{formatPrice(price)}</p>
      <p className={styles.pair}>BTC/USD</p>
      <output className={delayed ? styles.label : styles.hidden}>{delayed ? "Price delayed" : "Price live"}</output>
    </section>
  );
}
