import { seconds, test } from "../dsl";

test(
  "The test controls send a price, lose or restore the connection to the Price source, advance the clock, set the clock, and make the store fail resolutions or everything until it recovers; the clock stands still until a command moves it",
  // API only: the page does not show the server's time.
  { tag: "@api" },
  async ({ alice, priceSource, clock, store, server }) => {
    await alice.opensTheGame();

    await priceSource.sends("64,000.00");
    await alice.should.seeThePrice("64,000.00");
    await priceSource.losesTheConnection();
    await alice.should.seeThePriceDelayed("64,000.00");
    await priceSource.restoresTheConnection();
    await alice.should.seeThePrice("64,000.00");

    await clock.isSetTo("2027-03-01T08:30:00Z");
    await server.should.tellTheTime("2027-03-01T08:30:00Z");
    await server.should.tellTheTime("2027-03-01T08:30:00Z");

    await clock.advances(seconds(60));
    await server.should.tellTheTime("2027-03-01T08:31:00Z");

    await alice.guesses("Up");
    await store.failsResolutions();
    await clock.advances(seconds(60));
    await priceSource.sends("64,010.00");
    await alice.closesTheGame();
    await alice.opensTheGame();
    await alice.should.seeThePrice("64,010.00");
    await alice.should.haveAnOpenGuess("Up", "64,000.00");
    await alice.should.seeNoResult();
    await store.recovers();
    await alice.should.seeTheResult("Correct", "64,000.00", "64,010.00");

    await store.goesDown();
    await alice.should.beRefusedTheSnapshot("internal");
    await store.recovers();
    await alice.should.beServedTheSnapshot();
  },
);
