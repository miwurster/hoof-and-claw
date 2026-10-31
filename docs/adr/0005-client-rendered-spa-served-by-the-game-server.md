# A client-rendered React SPA, served by the game server

The frontend is a single-page app built with Vite, React and TypeScript, and it renders only in the browser.
After `vite build` it is static files with no server-side code of its own, and the game server serves them from the same Docker image and origin as the API (ADR 0001, ADR 0003).
The game server is the only backend, so every rule about prices and Outcomes lives in one place.
A second server that renders pages would show a price that is a second old by the time it arrives, so it adds nothing to a game with one screen and a live stream.

## Considered Options

- **A React meta-framework with server-side rendering: Next.js, Remix or React Router in framework mode, TanStack Start.** Rejected.
  Each one needs a Node runtime next to the Rust process, and its loaders and server functions would split server logic across two servers.
  React itself stays; only the framework layer with its own server is ruled out.
- **A CDN or a separate static host for the build.** Not chosen for the demo; it is on the scale path below.
  At demo scale the build is a few hundred kilobytes, cached by the browser after the first visit, so a CDN saves little and adds a second deployment to keep in step with the API.
- **No client-side router.** Rejected.
  The demo has one screen, but later pages such as a player profile, a leaderboard or a list of friends would each be a route.
  A router with a single route costs almost nothing now and spares a restructuring later.
- **React Router in declarative mode instead of TanStack Router.** Rejected.
  Its routes and parameters are only loosely typed, while TanStack Router turns a link to a missing route or a wrong parameter into a compile error, which suits code written by agents behind strict guardrails.
- **A data library such as TanStack Query, Redux or Zustand.** Rejected.
  The client reads its state once and then follows one event stream, which a reducer handles without a cache layer.

## Consequences

- **Serving.** File names with a content hash are served with `Cache-Control: public, max-age=31536000, immutable`.
  `index.html` is served `no-cache`, so the next page load after a deploy picks up the new build.
  Any path that is neither a file nor under `/api/` or `/test-controls/` falls back to `index.html`, so deep links work once there are more routes (ADR 0014).
- **Routing.** TanStack Router with routes defined in code, and for now the single route `/`.
  The demo uses no file-based routing and no route loaders.
- **Transport.** Each tab opens one SSE stream that carries prices and the events of its own Player, identified by the cookie.
  The server pushes a resolution event with the Outcome, End price and new Score the moment it resolves a guess, so the client never polls.
  HTTP is used for reading the Player's current state and for placing a Guess.
  The first event on every stream is a snapshot of the Player's state, so after a reconnect the client rebuilds its state from it (ADR 0010).
- **Countdown.** The server sends the time remaining on the Open guess as `remainingMs`, together with the server's current time (ADR 0014).
  The client counts down from when it received that message and never compares against its own clock.
  At zero the UI shows "Waiting for the price to move…" and decides nothing itself; only the server's resolution event ends the Open guess.
- **Client state.** One pure reducer takes the state and a server event or a user action and returns the new state.
  A thin hook owns the `EventSource` and the `fetch` calls and feeds the reducer.
- **Presentational components.** Components are small and receive only props and callbacks.
  They never fetch, open a stream or read the game hook; one container per route wires the hook to them.
  An architecture rule in the Green gate checks that presentational components do not import the data layer.
- **Test layers below the acceptance suite.** Unit tests are written test-first wherever they fit, not only for the reducer.
  The reducer, the hook with a fake `EventSource` and a fake clock, and every presentational component get their own unit tests with Vitest and Testing Library.
  The components can therefore be tested and verified on their own, with no server and no data layer.
  The acceptance suite's UI driver then covers the wiring between them.
- **Styling.** Colours, type, spacing, radii and motion are CSS custom properties, defined once in a token stylesheet that uses the names from `docs/design/visual-direction.md`.
  Each component has its own CSS Module and refers only to tokens.
  A Stylelint rule forbids raw colour values outside the token stylesheet.
  The demo stays dark-only, but a new theme or colour change means replacing token values in one place, for example under a `data-theme` attribute.

## Scale path

The static build moves off the game server when the price stream does, because outbound traffic is the first bottleneck (ADR 0003).
The options, in the order they become worth it:

- **A pull CDN in front of the Render origin**, for example Amazon CloudFront or Bunny CDN, on its own subdomain or path through a `CNAME`.
  It needs no change to the build and no move of DNS away from All-Inkl, and it only caches the hashed files while `index.html` and the API stay on the origin.
- **Amazon S3 with CloudFront**, once the platform moves to AWS.
  CloudFront serves the build from S3 and routes the API and the event stream to the application, so the browser still sees one origin and the cookie keeps working.
- **Cloudflare in front of everything.** This needs the whole `miwurster.com` zone moved from All-Inkl to Cloudflare, as ADR 0003 explains, so it only pays off if prices are delivered through Cloudflare as well.

Whichever option is chosen, frontend and API must keep sharing one origin for the identity cookie, or ADR 0001 has to be revisited.
