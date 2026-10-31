// The identity cookie as the Player's browser holds it.
import type { BrowserContext } from "@playwright/test";

const IDENTITY = "__Host-player";

/** The attributes of the identity cookie the browser holds, or `undefined` while it holds none. */
export async function identityCookie(context: BrowserContext) {
  const cookies = await context.cookies();
  const cookie = cookies.find((candidate) => candidate.name === IDENTITY);

  return cookie === undefined
    ? undefined
    : { path: cookie.path, httpOnly: cookie.httpOnly, secure: cookie.secure, sameSite: cookie.sameSite };
}
