# Anonymous player identity via an opaque cookie token, linkable to accounts later

The demo has no accounts, so the server knows a returning Player by an opaque random token of at least 128 bits, which it issues itself and stores only as a hash next to an internal Player ID.
The browser keeps the token in an `HttpOnly`, `Secure`, `SameSite=Lax` cookie with the `__Host-` prefix, served from the same origin as the API.
A cookie works with `EventSource`, which cannot set headers, keeps the token away from JavaScript, and needs no CSRF token because `SameSite=Lax` blocks cross-site POSTs.

## Considered Options

- **Signed token (HMAC or JWT) carrying the Player ID.** Rejected.
  Every request reads the Score anyway, so skipping the lookup gains nothing, and a signed token brings key management and cannot be revoked.
- **localStorage with an `Authorization` header.** Rejected.
  `EventSource` cannot send the header, and any script on the page can read the token.
- **A recovery code for a lost token.** Rejected.
  It is half an account system, and accounts are out of the demo.

## Consequences

- The server sets the cookie on the first visit but stores a Player only on the first Guess, so bots and drive-by visits spend none of the free tier's daily write budget.
  A token with no stored Player reads as Score 0 with no Open guess.
- The cookie lasts 400 days, the most Chrome allows, and is re-issued with a fresh expiry on each visit, so an active Player keeps it through the six-month demo.
  The server sets no expiry of its own.
- When the cookie is lost, so is the Player, and the browser starts over at 0.
  The UI says in one line that the Score lives in this browser.
- Frontend and API must share one origin, which constrains the hosting choice.
- An `Identity` port turns a request into a Player ID.
  The domain core never sees tokens or cookies, and the anonymous-cookie adapter is the demo's only implementation.
- When accounts arrive, the Player keeps owning the Score and Guesses, and each way of signing in is attached to a Player.
  The anonymous token is one such way, and a later login, for example an OIDC subject, is another.
  On first sign-in, an account without a Player takes over the browser's anonymous Player, including its Score and any Open guess.
  An account that already has a Player keeps it, and the anonymous Player is left behind.
  Scores are never merged, because summing them across browsers is gameable.
