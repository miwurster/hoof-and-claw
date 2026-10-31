import { milliseconds, seconds, test } from "../dsl";

test("A Guess does not resolve before 60 seconds have passed, even when the price changes", async ({
  alice,
  priceSource,
  clock,
}) => {
  await priceSource.sends("64,000.00");
  await alice.opensTheGame();
  await alice.guesses("Up");

  await clock.advances(seconds(30));
  await priceSource.sends("64,010.00");

  await alice.should.seeThePrice("64,010.00");
  await alice.should.haveAnOpenGuess("Up", "64,000.00");
  await alice.should.haveScore(0);
});

test("A Guess stays open after 60 seconds for as long as every received price equals the Start price", async ({
  alice,
  priceSource,
  clock,
}) => {
  await priceSource.sends("64,000.00");
  await alice.opensTheGame();
  await alice.guesses("Down");

  await clock.advances(seconds(60));
  await priceSource.sends("64,000.00");
  await clock.advances(seconds(30));
  await priceSource.sends("64,000.00");

  await alice.should.haveAnOpenGuess("Down", "64,000.00");

  await priceSource.sends("63,999.99");

  await alice.should.seeTheResult("Correct", "64,000.00", "63,999.99");
});

test("A price received 59.999 seconds after the Guess does not resolve it; a changed price received at exactly 60 seconds does", async ({
  alice,
  priceSource,
  clock,
}) => {
  await priceSource.sends("64,000.00");
  await alice.opensTheGame();
  await alice.guesses("Up");

  await clock.advances(milliseconds(59_999));
  await priceSource.sends("64,010.00");

  await alice.should.seeThePrice("64,010.00");
  await alice.should.haveAnOpenGuess("Up", "64,000.00");

  await clock.advances(milliseconds(1));
  await priceSource.sends("64,020.00");

  await alice.should.seeTheResult("Correct", "64,000.00", "64,020.00");
});

test("The End price is the first price received at least 60 seconds after the Guess that differs from the Start price by at least one cent", async ({
  alice,
  priceSource,
  clock,
}) => {
  await priceSource.sends("64,000.00");
  await alice.opensTheGame();
  await alice.guesses("Up");

  await clock.advances(seconds(30));
  await priceSource.sends("63,900.00");
  await clock.advances(seconds(30));
  await priceSource.sends("64,000.00");
  await priceSource.sends("64,000.01");
  await priceSource.sends("63,950.00");

  await alice.should.seeTheResult("Correct", "64,000.00", "64,000.01");
});

test("A Guess Up resolves Correct when the End price is higher, and the Score rises by 1", async ({
  alice,
  priceSource,
  clock,
}) => {
  await priceSource.sends("64,000.00");
  await alice.opensTheGame();
  await alice.guesses("Up");

  await clock.advances(seconds(60));
  await priceSource.sends("64,012.40");

  await alice.should.seeTheResult("Correct", "64,000.00", "64,012.40");
  await alice.should.haveScore(1);
});

test("A Guess Down resolves Correct when the End price is lower, and the Score rises by 1", async ({
  alice,
  priceSource,
  clock,
}) => {
  await priceSource.sends("64,000.00");
  await alice.opensTheGame();
  await alice.guesses("Down");

  await clock.advances(seconds(60));
  await priceSource.sends("63,990.00");

  await alice.should.seeTheResult("Correct", "64,000.00", "63,990.00");
  await alice.should.haveScore(1);
});

test("A Guess Up resolves Wrong when the End price is lower, and the Score falls by 1", async ({
  alice,
  priceSource,
  clock,
}) => {
  await priceSource.sends("64,000.00");
  await alice.opensTheGame();
  await alice.guesses("Up");
  await clock.advances(seconds(60));
  await priceSource.sends("64,010.00");
  await alice.should.haveScore(1);
  await alice.guesses("Up");

  await clock.advances(seconds(60));
  await priceSource.sends("64,000.00");

  await alice.should.seeTheResult("Wrong", "64,010.00", "64,000.00");
  await alice.should.haveScore(0);
});

test("A Guess Down resolves Wrong when the End price is higher, and the Score falls by 1", async ({
  alice,
  priceSource,
  clock,
}) => {
  await priceSource.sends("64,000.00");
  await alice.opensTheGame();
  await alice.guesses("Down");
  await clock.advances(seconds(60));
  await priceSource.sends("63,990.00");
  await alice.should.haveScore(1);
  await alice.guesses("Down");

  await clock.advances(seconds(60));
  await priceSource.sends("64,000.00");

  await alice.should.seeTheResult("Wrong", "63,990.00", "64,000.00");
  await alice.should.haveScore(0);
});

test("A Score can go below 0", async ({ alice, priceSource, clock }) => {
  await priceSource.sends("64,000.00");
  await alice.opensTheGame();
  await alice.guesses("Up");
  await clock.advances(seconds(60));
  await priceSource.sends("63,990.00");
  await alice.should.haveScore(-1);
  await alice.guesses("Up");

  await clock.advances(seconds(60));
  await priceSource.sends("63,980.00");

  await alice.should.seeTheResult("Wrong", "63,990.00", "63,980.00");
  await alice.should.haveScore(-2);
});

test("A resolved Guess shows its Start price, End price, Outcome and the Score change, and stays until the next Guess", async ({
  alice,
  priceSource,
  clock,
}) => {
  await priceSource.sends("64,000.00");
  await alice.opensTheGame();
  await alice.guesses("Down");
  await clock.advances(seconds(60));
  await priceSource.sends("64,050.00");

  await alice.should.seeTheResult("Wrong", "64,000.00", "64,050.00");

  await clock.advances(seconds(60));
  await priceSource.sends("64,060.00");
  await alice.should.seeThePrice("64,060.00");

  await alice.should.seeTheResult("Wrong", "64,000.00", "64,050.00");

  await alice.guesses("Up");

  await alice.should.haveAnOpenGuess("Up", "64,060.00");
  await alice.should.seeNoResult();
});

test(
  "A screen reader announces the Outcome politely, and does not announce the running price",
  // UI only: a screen reader reads the page.
  { tag: "@ui" },
  async ({ alice, priceSource, clock }) => {
    await priceSource.sends("64,000.00");
    await alice.opensTheGame();
    await alice.guesses("Up");

    await clock.advances(seconds(60));
    await priceSource.sends("64,010.00");

    await alice.should.hearTheOutcomeAnnouncedAs("Correct");
  },
);

test("An Open guess that falls due while the price is delayed resolves against the first changed price received after the price recovers", async ({
  alice,
  priceSource,
  clock,
}) => {
  await priceSource.sends("64,000.00");
  await alice.opensTheGame();
  await alice.guesses("Up");

  await priceSource.losesTheConnection();
  await clock.advances(seconds(90));
  await alice.should.seeThePriceDelayed("64,000.00");
  await alice.should.haveAnOpenGuess("Up", "64,000.00");

  await priceSource.restoresTheConnection();
  await priceSource.sends("64,000.00");
  await priceSource.sends("63,900.00");

  await alice.should.seeTheResult("Wrong", "64,000.00", "63,900.00");
  await alice.should.haveScore(-1);
});

test("A Guess resolves while the Player has no page open, and on return the Player sees its Outcome and the new Score", async ({
  alice,
  priceSource,
  clock,
}) => {
  await priceSource.sends("64,000.00");
  await alice.opensTheGame();
  await alice.guesses("Up");
  await alice.should.haveAnOpenGuess("Up", "64,000.00");

  await alice.closesTheGame();
  await clock.advances(seconds(60));
  await priceSource.sends("64,100.00");
  await alice.opensTheGame();

  await alice.should.seeTheResult("Correct", "64,000.00", "64,100.00");
  await alice.should.haveScore(1);
});

test("Two Players do not affect each other: Bob's Guess and Outcome change nothing for Alice", async ({
  alice,
  bob,
  priceSource,
  clock,
}) => {
  await priceSource.sends("64,000.00");
  await alice.opensTheGame();
  await bob.opensTheGame();

  await bob.guesses("Down");

  await bob.should.haveAnOpenGuess("Down", "64,000.00");
  await alice.should.haveNoOpenGuess();
  await alice.should.beAbleToGuess();

  await clock.advances(seconds(60));
  await priceSource.sends("63,900.00");

  await bob.should.seeTheResult("Correct", "64,000.00", "63,900.00");
  await bob.should.haveScore(1);
  await alice.should.seeThePrice("63,900.00");
  await alice.should.haveScore(0);
  await alice.should.haveNoOpenGuess();
  await alice.should.seeNoResult();
});

test("A trade at the same price as the last one still updates the latest price and runs resolution, but sends no price event to the browser", async ({
  alice,
  priceSource,
  clock,
}) => {
  // That no price event goes out is proven in `game` with the recording Fanout; this case proves the resolution.
  await priceSource.sends("64,000.00");
  await alice.opensTheGame();
  await alice.guesses("Up");
  await clock.advances(seconds(30));
  await priceSource.sends("64,010.00");
  await alice.should.seeThePrice("64,010.00");

  await clock.advances(seconds(30));
  await priceSource.sends("64,010.00");

  await alice.should.seeTheResult("Correct", "64,000.00", "64,010.00");
});
