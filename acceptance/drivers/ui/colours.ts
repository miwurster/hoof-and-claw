// The page's colours, read from the design tokens rather than repeated here.
import { readFileSync } from "node:fs";
import { join } from "node:path";

/** The design tokens' stylesheet, the one place that names the page's colours. */
const TOKENS = readFileSync(join(import.meta.dirname, "..", "..", "..", "spa", "src", "styles", "tokens.css"), "utf8");

/** A colour token as `getComputedStyle` writes it, such as `rgb(212, 178, 106)`. */
export function colour(token: string): string {
  const hex = new RegExp(`${token}:\\s*#([0-9a-f]{6});`, "iu").exec(TOKENS)?.[1];

  if (hex === undefined) {
    throw new Error(`the token stylesheet has no colour ${token}`);
  }

  const [red, green, blue] = [0, 2, 4].map((at) => Number.parseInt(hex.slice(at, at + 2), 16));

  return `rgb(${String(red)}, ${String(green)}, ${String(blue)})`;
}
