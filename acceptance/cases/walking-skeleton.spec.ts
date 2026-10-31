import { test } from "../dsl";

test(
  "The server reports its build and its test controls once migrations ran, also while the price is delayed",
  // API only: the health check is not part of the page a Player sees.
  { tag: "@api" },
  async ({ server }) => {
    await server.should.reportTheBuildItWasLaunchedFromWithItsTestControls();
  },
);

test(
  "A route of the game or the test controls that does not exist is not found",
  // API only: a Player's page never asks for a missing route.
  { tag: "@api" },
  async ({ server }) => {
    await server.should.findNothingAtAMissingGameRoute();
    await server.should.findNothingAtAMissingTestControl();
  },
);

test("Any other unknown path serves the app", async ({ alice }) => {
  await alice.opensAnUnknownPage();

  await alice.should.seeTheGame();
});
