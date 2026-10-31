import { test } from "../dsl";

test(
  "When the stream is lost, the page shows the last price as Price delayed with Up and Down disabled, reconnects after a random, growing delay, and rebuilds its state from the new snapshot",
  // UI only: reconnecting is the page's own behaviour, and the API has no page.
  { tag: "@ui" },
  async ({ alice, priceSource }) => {
    await priceSource.sends("64,000.00");
    await alice.opensTheGame();
    await alice.should.seeThePrice("64,000.00");

    await alice.losesTheNetwork();

    await alice.should.seeThePriceDelayed("64,000.00");
    await alice.should.notBeAbleToGuess();

    await priceSource.sends("64,100.00");
    await alice.regainsTheNetwork();

    await alice.should.seeThePrice("64,100.00");
    await alice.should.beAbleToGuess();
  },
);

test(
  "An IP may hold at most 10 streams at once; the eleventh is refused with rateLimited",
  // API only: a page holds one stream, so only the API can open eleven from one IP.
  { tag: "@api" },
  async ({ alice }) => {
    await alice.opensTheGameInTabs(10);

    await alice.should.beRefusedAnotherTab("rateLimited");
  },
);
