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
