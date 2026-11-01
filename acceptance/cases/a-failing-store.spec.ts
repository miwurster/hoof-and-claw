import { seconds, test } from "../dsl";

test("A resolution is published only after its save: while the store fails resolutions the due Guess shows no Outcome, and once the store recovers the Outcome shows the End price of the price that resolved it", async ({
  alice,
  priceSource,
  clock,
  store,
}) => {
  await priceSource.sends("64,000.00");
  await alice.opensTheGame();
  await alice.guesses("Up");

  await store.failsResolutions();
  await clock.advances(seconds(60));
  await priceSource.sends("64,010.00");
  await priceSource.sends("64,020.00");

  await alice.closesTheGame();
  await alice.opensTheGame();
  await alice.should.seeThePrice("64,010.00");
  await alice.should.haveAnOpenGuess("Up", "64,000.00");
  await alice.should.seeNoResult();

  await store.recovers();

  await alice.should.seeTheResult("Correct", "64,000.00", "64,010.00");
  await alice.should.haveScore(1);
});

test("A price received while a failed save of resolutions is retried keeps the time it arrived at, so it never becomes the End price of a Guess that was not yet due then", async ({
  alice,
  bob,
  priceSource,
  clock,
  store,
}) => {
  await priceSource.sends("64,000.00");
  await alice.opensTheGame();
  await bob.opensTheGame();
  await alice.guesses("Up");
  await clock.advances(seconds(30));
  await bob.guesses("Up");

  await store.failsResolutions();
  await clock.advances(seconds(30));
  await priceSource.sends("63,990.00");
  await priceSource.sends("64,010.00");
  await clock.advances(seconds(35));
  await store.recovers();

  await bob.should.haveAnOpenGuess("Up", "64,000.00");

  await priceSource.sends("64,020.00");

  await bob.should.seeTheResult("Correct", "64,000.00", "64,020.00");
});

test("While the store is down a Guess is not placed; once the store recovers the next Guess is placed", async ({
  alice,
  priceSource,
  store,
}) => {
  await priceSource.sends("64,000.00");
  await alice.opensTheGame();
  await alice.should.seeThePrice("64,000.00");

  await store.goesDown();
  await alice.guesses("Up");

  await alice.should.haveTheGuessRefusedAs("internal");
  await alice.should.haveNoOpenGuess();

  await store.recovers();
  await alice.guesses("Up");

  await alice.should.haveAnOpenGuess("Up", "64,000.00");
});

test(
  "While the store is down the snapshot and the stream answer internal; both deliver again once the store recovers",
  // API only: the page reconnects a refused stream by itself and never reads the snapshot alone.
  { tag: "@api" },
  async ({ alice, priceSource, store }) => {
    await priceSource.sends("64,000.00");
    await alice.opensTheGame();
    await alice.guesses("Up");
    await alice.closesTheGame();

    await store.goesDown();

    await alice.should.beRefusedTheGame("internal");
    await alice.should.beRefusedTheSnapshot("internal");

    await store.recovers();

    await alice.should.beServedTheSnapshot();
    await alice.opensTheGame();
    await alice.should.haveAnOpenGuess("Up", "64,000.00");
  },
);

test(
  "A page opened while the store is down shows Price delayed with Up and Down disabled and keeps reconnecting; once the store recovers it shows the Score and the Open guess",
  // UI only: only the page reconnects by itself and disables Up and Down.
  { tag: "@ui" },
  async ({ alice, priceSource, clock, store }) => {
    await priceSource.sends("64,000.00");
    await alice.opensTheGame();
    await alice.guesses("Up");
    await clock.advances(seconds(60));
    await priceSource.sends("64,010.00");
    await alice.should.haveScore(1);
    await alice.guesses("Down");
    await alice.should.haveAnOpenGuess("Down", "64,010.00");
    await alice.closesTheGame();

    await store.goesDown();
    await alice.opensTheGame();

    await alice.should.seeThePriceDelayed(null);
    await alice.should.notBeAbleToGuess();
    await alice.should.keepReconnecting();

    await store.recovers();

    await alice.should.getTheGameBack();
    await alice.should.seeThePrice("64,010.00");
    await alice.should.haveScore(1);
    await alice.should.haveAnOpenGuess("Down", "64,010.00");
  },
);
