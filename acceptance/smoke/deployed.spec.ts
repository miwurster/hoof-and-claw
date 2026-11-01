// `pnpm smoke`, run by hand after every Release against the deployed service (ADR 0013).
import { test } from "../dsl/deployed";

test(
  "The deployed page loads as the game",
  // UI only: the API has no page.
  { tag: "@ui" },
  async ({ alice }) => {
    await alice.opensTheGame();

    await alice.should.seeTheGame();
  },
);

test("A Reviewer opening the deployed game sees a live price from its stream", async ({ alice }) => {
  await alice.opensTheGame();

  await alice.should.seeALivePrice();
});

test(
  "The deployed service reports the released commit",
  // API only: the health check is not part of the page a Player sees.
  { tag: "@api" },
  async ({ server }) => {
    await server.should.reportTheReleasedBuild();
  },
);

test(
  "A forged X-Forwarded-For does not lift the deployed limit of 10 streams per IP",
  // API only: a page holds one stream, so only the API can open eleven from one IP.
  // Alice and Bob run on this machine, so they share its IP; each claims a network of their own in X-Forwarded-For.
  // Run it apart from `pnpm smoke:hold`, whose stream takes one of the 10 places.
  { tag: "@api" },
  async ({ alice, bob }) => {
    await alice.opensTheGameInTabs(10);

    await bob.should.beRefusedTheGame("rateLimited");
  },
);
