import { seconds, test } from "../dsl";

test(
  "The test controls send a price, lose or restore the connection to the Price source, advance the clock, and set the clock; the clock stands still until a command moves it",
  // API only: the page does not show the server's time.
  { tag: "@api" },
  async ({ alice, priceSource, clock, server }) => {
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
  },
);
