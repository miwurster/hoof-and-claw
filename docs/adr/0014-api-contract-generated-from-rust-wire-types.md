# API contract: JSON over three routes, one tagged event union, TypeScript types generated from Rust

The SPA and the game server talk over three routes under `/api/`: a read of the snapshot, the command to place a Guess, and the SSE stream.
Every SSE event is one JSON object with a `type` field, so the client's reducer handles one discriminated union.
The Rust serde types of the wire format are the only source of truth; `ts-rs` generates the TypeScript types from them into a pnpm package, `contract/`, which the SPA and the acceptance suite import.
This settles what ADR 0005, ADR 0006, ADR 0010, ADR 0011 and ADR 0012 left open about the API contract between frontend and server.

## Decision

### Routes

- Everything the SPA calls lives under `/api/`, with no version prefix, because the SPA and the server are built from one commit and deployed together.
- `GET /api/snapshot` returns the snapshot, the same object as the stream's first event.
  It is ADR 0010's HTTP read of the Player's state.
- `POST /api/guesses` places a Guess.
- `GET /api/events` is the SSE stream.
- The test controls live apart, under `/test-controls/`, and exist only in a build with the Cargo feature `test-controls` (ADR 0008).
- Paths under `/api/` and `/test-controls/` never fall back to `index.html` (ADR 0005); an unknown one returns `404` with `{ "error": "notFound" }`.
- A middleware sets the identity cookie on every `/api/` response and on `index.html` when the request has none, and re-issues it with a fresh expiry otherwise (ADR 0001).
- A visitor without a stored Player gets Score 0 and no Open guess, and the server writes nothing (ADR 0001).

### Wire format

- JSON field names are camelCase.
- Every price is an integer number of cents (ADR 0009), such as `startPrice: 6400012`; it stays well inside a JavaScript `number`'s safe range.
  Only the client formats dollars.
- `direction` is `"up"` or `"down"`; `outcome` is `"correct"` or `"wrong"`.
- A Guess ID is an opaque string.
- The time left on an Open guess is `remainingMs`, a whole number of milliseconds that is 0 once the Guess is due.
- `serverTime` is an RFC 3339 timestamp.
- `price` is `null` while the server has not received a price since it started; `delayed` is then `true`.
- `build` is the commit the server was built from.

### Payloads

```ts
type OpenGuess = {
  id: string;
  direction: Direction;
  startPrice: number;
  remainingMs: number;
};
type ResolvedGuess = {
  id: string;
  direction: Direction;
  startPrice: number;
  endPrice: number;
  outcome: Outcome;
};

type Snapshot = {
  type: "snapshot";
  build: string;
  serverTime: string;
  price: number | null;
  delayed: boolean;
  score: number;
  openGuess: OpenGuess | null;
  lastResolved: ResolvedGuess | null;
};
type PriceChanged = { type: "price"; price: number | null; delayed: boolean };
type GuessPlaced = { type: "guessPlaced"; guess: OpenGuess };
type GuessResolved = {
  type: "guessResolved";
  guess: ResolvedGuess;
  score: number;
};

type ServerEvent = Snapshot | PriceChanged | GuessPlaced | GuessResolved;
```

- `lastResolved` is the Player's most recent Resolved guess, so a tab that was away when a Guess resolved still shows its Outcome, as ADR 0010 requires of the snapshot.
- A change between live and Price delayed is a `price` event with the new `delayed`, not an event of its own, because every event carries absolute values (ADR 0010).
- `GET /api/snapshot` returns the `Snapshot` object, `type` included.

### SSE framing

- A game event has no `event:` field and no `id:` field; its `data:` line holds the JSON object.
  Every game event reaches `EventSource`'s `message` handler, and the reducer switches on `type`.
- The server sends no `retry:` field.
- The keepalive is the event `keepalive` every 15 s, with `event: keepalive` and `data: {}` (ADR 0010).
  It reaches only a `keepalive` listener, never the `message` handler, and its data means nothing; `EventSource` drops an event without data.

### Placing a Guess

- The request is `POST /api/guesses` with `Content-Type: application/json` and the body `{ "direction": "up" }`.
  A cross-site form cannot send that content type without a CORS preflight, a second guard beside the `SameSite=Lax` cookie (ADR 0001).
- On success the server answers `201` with the `OpenGuess`, after it has saved the Guess and published Guess placed (ADR 0010).
  The reducer takes the Open guess by its ID, so the answer and the `guessPlaced` event can arrive in either order.

### Errors

Every error from a route under `/api/` or `/test-controls/` is `{ "error": "<code>" }`, with the code from one generated enum:

| Status | Code               | When                                                                      |
| ------ | ------------------ | ------------------------------------------------------------------------- |
| 409    | `openGuessExists`  | the Player already has an Open guess                                      |
| 409    | `priceDelayed`     | the price is Price delayed, or no price has been received yet (ADR 0007)  |
| 422    | `invalidDirection` | the direction is neither `up` nor `down`                                  |
| 400    | `badRequest`       | the body is not valid JSON, or the content type is not `application/json` |
| 404    | `notFound`         | no such route                                                             |
| 429    | `rateLimited`      | a rate limit or the cap on streams per IP is reached                      |
| 500    | `internal`         | anything else, with no detail in the body                                 |

`web` maps the axum extractors' rejections and `tower_governor`'s answer into this format, so no route under `/api/` answers with plain text.
A refused Guess is a `409` and not a `503`, because the server is healthy and a rule of the game refuses the Guess.

### Sharing the types

- The wire types are serde structs and enums in `web`, separate from the domain types of the cores, and `#[derive(TS)]` from `ts-rs` exports them.
  The test controls' request types live in `test-controls` and are exported the same way.
- The generated files go into the pnpm package `contract/` and are checked in.
  The game's types are its main export; the test controls' types are a second export, `contract/test-controls`.
- The Green gate regenerates the types and fails if the checked-in files differ.
- `spa/` imports only the main export.
  In `acceptance/`, only `drivers/api/` imports the main export and only `drivers/controls/` imports `contract/test-controls`.

### The client side

- The hook decodes every response body and every event in one place, with one `as` to the generated type.
  Its `// SAFETY:` comment says that the SPA and the server come from the same commit and the same origin.
  There is no schema library and no runtime validation.
- Every snapshot carries `build`.
  If it differs from the SPA's own build, the SPA reloads the page instead of running an old client against a new contract.
- The hook alone reconnects the stream.
  On every `error` event and after 45 s without any data (ADR 0010), it closes the `EventSource` and opens a new one after a random delay with exponential backoff.
  The browser's own reconnect is never used.

## Considered Options

- **OpenAPI with `utoipa` and a generated client.** Rejected.
  OpenAPI describes SSE poorly, and a generated client duplicates the hook, which already owns every `fetch` and the stream.
- **TypeScript types written by hand, checked by contract tests.** Rejected.
  Two sources of truth drift, and the tests only catch the drift they were written for.
- **Runtime validation with Valibot or Zod.** Rejected.
  Its schemas would be a second source of truth beside the generated types, against a server from the same commit and origin.
  The `build` check already handles the one case it would catch, a tab left open across a Release.
- **Allowing only additive changes to the contract.** Rejected.
  The gate cannot enforce it, and the `build` check reloads every old client anyway.
- **Named SSE events.** Rejected.
  They need one `addEventListener` per name and split the reducer's input, while a `type` field gives one discriminated union from one serde enum.
- **A separate event for Price delayed.** Rejected, because the `price` event already carries the state as an absolute value.
- **The generated types inside `spa/`, imported by `acceptance/`.** Rejected, because one package would reach into another's sources.
- **RFC 9457 `application/problem+json` for errors.** Rejected for the demo, because with one client its fields would be ceremony.
  It is the format to migrate to when the API gains other clients on the scale path; the codes become its `type` values.
- **`503` while the price is delayed.** Rejected, because it tells clients and monitors that the server is down.
- **The browser's own reconnect, alone or alongside the hook.** Rejected.
  `EventSource` gives up for good on any status that is not 2xx, such as a `429` at the stream cap or a `502` during a Render deploy.
  It also reconnects with no random delay, so every tab reconnects at once after a deploy, against ADR 0010's scale path, and it never notices a stream that has gone silent.
  Using both would give two reconnect paths with different timing and a risk of duplicate streams.

## Consequences

- ADR 0005's countdown uses `remainingMs`, with `serverTime` beside it, and its fallback to `index.html` excludes `/api/` and `/test-controls/`.
- ADR 0010's open question about event names and payloads is settled here.
- ADR 0011's API driver and controls driver build on the generated types.
- `web` and `test-controls` depend on `serde` and `ts-rs`.
- The hook must close the old `EventSource` before it opens a new one; a forgotten `close()` would let the browser reconnect alongside and count twice against the streams per IP.
  A hook test with the fake `EventSource` covers it.
