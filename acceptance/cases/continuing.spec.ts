import { seconds, test } from "../dsl";

test("An Open guess survives a server restart and resolves against the first changed price received after it", async ({
  alice,
  priceSource,
  clock,
  server,
}) => {
  await priceSource.sends("64,000.00");
  await alice.opensTheGame();
  await alice.guesses("Up");
  await alice.closesTheGame();

  await server.restarts();
  await clock.advances(seconds(60));
  await priceSource.sends("64,000.00");
  await priceSource.sends("64,000.50");
  await priceSource.sends("63,990.00");
  await alice.opensTheGame();

  await alice.should.seeTheResult("Correct", "64,000.00", "64,000.50");
  await alice.should.haveScore(1);
});

test("A Player who closes the browser and returns sees the same Score and the same Open guess, and the countdown continues from the server's remaining time", async ({
  alice,
  priceSource,
  clock,
}) => {
  await priceSource.sends("64,000.00");
  await alice.opensTheGame();
  await alice.guesses("Up");
  await clock.advances(seconds(60));
  await priceSource.sends("64,000.50");
  await alice.should.haveScore(1);
  await alice.guesses("Down");
  await clock.advances(seconds(20));

  await alice.closesTheGame();
  await clock.advances(seconds(15));
  await alice.opensTheGame();

  await alice.should.haveScore(1);
  await alice.should.haveAnOpenGuess("Down", "64,000.50");
  await alice.should.seeSecondsLeft(25);
});

test("A Guess placed in one tab appears as the Open guess in the Player's other tabs", async ({
  alice,
  priceSource,
}) => {
  await priceSource.sends("64,000.00");
  await alice.opensTheGame();
  const otherTab = await alice.opensAnotherTab();

  await alice.guesses("Up");

  await otherTab.should.haveAnOpenGuess("Up", "64,000.00");
});

test("A resolution reaches every open tab of the Player with the same Outcome and Score", async ({
  alice,
  priceSource,
  clock,
}) => {
  await priceSource.sends("64,000.00");
  await alice.opensTheGame();
  const otherTab = await alice.opensAnotherTab();
  await alice.guesses("Down");

  await clock.advances(seconds(60));
  await priceSource.sends("63,999.00");

  await alice.should.seeTheResult("Correct", "64,000.00", "63,999.00");
  await alice.should.haveScore(1);
  await otherTab.should.seeTheResult("Correct", "64,000.00", "63,999.00");
  await otherTab.should.haveScore(1);
});

// Refusing new Guesses, letting running requests finish and waiting for a running save happen inside the shutdown,
// where no Player can watch them; `game`'s tests prove them (crates/game/tests/continuing.rs).
test("On SIGTERM, the server closes the Bitstamp connection and publishes Price delayed, refuses new Guesses with priceDelayed, lets running requests finish, ends every stream, waits for a running resolution save, and exits within 20 seconds", async ({
  alice,
  priceSource,
  server,
}) => {
  await priceSource.sends("64,000.00");
  await alice.opensTheGame();
  await alice.should.seeThePrice("64,000.00");

  await server.stops();

  await server.should.haveStoppedWithin(seconds(20));
  await alice.should.seeThePriceDelayed("64,000.00");
  await alice.should.notBeAbleToGuess();
});

test("A Player who is watching during a restart sees Price delayed, then the price again after reconnecting, with the same Score and Open guess", async ({
  alice,
  priceSource,
  clock,
  server,
}) => {
  await priceSource.sends("64,000.00");
  await alice.opensTheGame();
  await alice.guesses("Up");
  await clock.advances(seconds(60));
  await priceSource.sends("64,000.50");
  await alice.should.haveScore(1);
  await alice.guesses("Down");

  await server.stops();
  await alice.should.seeThePriceDelayed("64,000.50");
  await server.startsAgain();
  await priceSource.sends("64,000.50");

  await alice.should.seeThePrice("64,000.50");
  await alice.should.haveScore(1);
  await alice.should.haveAnOpenGuess("Down", "64,000.50");
});
