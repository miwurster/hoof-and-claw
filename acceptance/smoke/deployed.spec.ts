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
