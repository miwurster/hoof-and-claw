// How prices and the Score read on the page: always en-US, with a true minus sign.
const dollars = new Intl.NumberFormat("en-US", { style: "currency", currency: "USD" });

/** A price in cents as US dollars with cents, such as `$67,412.18`, or a dash while there is none. */
export function formatPrice(cents: number | null): string {
  return cents === null ? "—" : dollars.format(cents / 100);
}

/** The Score with a true minus sign, such as `Score −2`. */
export function formatScore(score: number): string {
  return `Score ${score < 0 ? `−${String(-score)}` : String(score)}`;
}

/** The difference of the latest price from the Start price, such as `+$12.40 ↑`, or `$0.00` with no arrow. */
export function formatDifference(cents: number): string {
  if (cents === 0) {
    return dollars.format(0);
  }

  const amount = dollars.format(Math.abs(cents) / 100);

  return cents > 0 ? `+${amount} ↑` : `−${amount} ↓`;
}

/** The time left on an Open guess in whole seconds, rounded up, so the last second shows 1 until it is over. */
export function formatSeconds(milliseconds: number): string {
  return String(Math.ceil(milliseconds / 1000));
}
