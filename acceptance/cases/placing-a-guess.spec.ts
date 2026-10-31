import { seconds, test } from "../dsl";

test("A Player places a Guess Up and sees it as their Open guess with Direction Up and the Start price", async ({
  alice,
  priceSource,
}) => {
  await priceSource.sends("64,000.00");
  await alice.opensTheGame();
  await alice.should.seeThePrice("64,000.00");

  await alice.guesses("Up");

  await alice.should.haveAnOpenGuess("Up", "64,000.00");
});

test("A Player places a Guess Down and sees it as their Open guess with Direction Down and the Start price", async ({
  alice,
  priceSource,
}) => {
  await priceSource.sends("64,000.00");
  await alice.opensTheGame();
  await alice.should.seeThePrice("64,000.00");

  await alice.guesses("Down");

  await alice.should.haveAnOpenGuess("Down", "64,000.00");
});

test("The Start price is the last price the server received before it accepted the Guess", async ({
  alice,
  priceSource,
}) => {
  await priceSource.sends("64,000.00");
  await alice.opensTheGame();
  await alice.should.seeThePrice("64,000.00");

  // Within the same second as the last price, so the page may not show it yet.
  await priceSource.sends("64,000.50");
  await alice.guesses("Up");

  await alice.should.haveAnOpenGuess("Up", "64,000.50");
});

test(
  "While a Guess is being placed, Up and Down are disabled until the answer or the Guess placed event arrives",
  // UI only: the API has no buttons.
  { tag: "@ui" },
  async ({ alice, priceSource }) => {
    await priceSource.sends("64,000.00");
    await alice.opensTheGame();
    await alice.should.beAbleToGuess();

    await alice.startsToGuess("Up");

    await alice.should.notBeAbleToGuess();
    await alice.should.haveNoOpenGuess();

    await alice.receivesTheAnswer();

    await alice.should.haveAnOpenGuess("Up", "64,000.00");
    await alice.should.notBeAbleToGuess();
  },
);

test("The Open guess shows the remaining seconds, counted down from the server's remaining time", async ({
  alice,
  priceSource,
  clock,
}) => {
  await priceSource.sends("64,000.00");
  await alice.opensTheGame();
  await alice.should.seeThePrice("64,000.00");
  await alice.guesses("Up");
  await alice.should.seeSecondsLeft(60);

  await clock.advances(seconds(20));
  await alice.opensTheGame();

  await alice.should.seeSecondsLeft(40);
});

test(
  "The Open guess shows the live difference from the Start price, coloured by whether the Guess would currently be Correct, and $0.00 in Flint with no arrow when the price equals the Start price",
  // UI only: the API carries prices, and only the page shows the difference and its colour.
  { tag: "@ui" },
  async ({ alice, priceSource }) => {
    await priceSource.sends("64,000.00");
    await alice.opensTheGame();
    await alice.guesses("Up");

    await alice.should.seeTheDifference("$0.00", "Unchanged");

    await priceSource.sends("64,012.40");

    await alice.should.seeTheDifference("+$12.40 ↑", "Correct");

    await priceSource.sends("63,990.00");

    await alice.should.seeTheDifference("−$10.00 ↓", "Wrong");

    await priceSource.sends("64,000.00");

    await alice.should.seeTheDifference("$0.00", "Unchanged");
  },
);

test(
  'After 60 seconds without a changed price, the Open guess shows "Waiting for the price to move…"',
  // UI only: the API carries the remaining time, and only the page shows the waiting label.
  { tag: "@ui" },
  async ({ alice, priceSource, clock }) => {
    await priceSource.sends("64,000.00");
    await alice.opensTheGame();
    await alice.guesses("Down");

    await clock.advances(seconds(60));
    await priceSource.sends("64,000.00");
    await alice.opensTheGame();

    await alice.should.seeTheGuessWaitingForThePriceToMove();
    await alice.should.haveAnOpenGuess("Down", "64,000.00");
  },
);

test("A Player with an Open guess cannot place another", async ({ alice, priceSource }) => {
  await priceSource.sends("64,000.00");
  await alice.opensTheGame();
  await alice.guesses("Up");
  await alice.should.haveAnOpenGuess("Up", "64,000.00");

  await alice.should.notBeAbleToPlaceAnotherGuess();
  await alice.should.haveAnOpenGuess("Up", "64,000.00");
});

test(
  "The place-a-Guess command takes only a Direction; a request that carries a price or a time places the Guess with the server's own Start price and time",
  // API only: the page sends only a Direction.
  { tag: "@api" },
  async ({ alice, priceSource }) => {
    await priceSource.sends("64,000.00");
    await alice.opensTheGame();

    await alice.guessesWithAPriceAndTimeOfTheirOwn("Up");

    await alice.should.haveAnOpenGuess("Up", "64,000.00");
    await alice.should.seeSecondsLeft(60);
  },
);

test(
  "A request with a Direction other than Up or Down is refused with invalidDirection",
  // API only: the page has only Up and Down.
  { tag: "@api" },
  async ({ alice, priceSource }) => {
    await priceSource.sends("64,000.00");
    await alice.opensTheGame();

    await alice.guessesADirectionThatIsNeitherUpNorDown();

    await alice.should.haveTheGuessRefusedAs("invalidDirection");
    await alice.should.haveNoOpenGuess();
  },
);

test(
  "A request whose body is not JSON, or whose content type is not application/json, is refused with badRequest",
  // API only: the page always sends JSON.
  { tag: "@api" },
  async ({ alice, priceSource }) => {
    await priceSource.sends("64,000.00");
    await alice.opensTheGame();

    await alice.guessesWithABodyThatIsNotJson();

    await alice.should.haveTheGuessRefusedAs("badRequest");

    await alice.guessesWithoutSayingTheBodyIsJson();

    await alice.should.haveTheGuessRefusedAs("badRequest");
    await alice.should.haveNoOpenGuess();
  },
);

test(
  "While the price is delayed, the API refuses a Guess with priceDelayed",
  // API only: the page disables Up and Down instead.
  { tag: "@api" },
  async ({ alice, priceSource }) => {
    await priceSource.sends("64,000.00");
    await alice.opensTheGame();
    await priceSource.losesTheConnection();
    await alice.should.seeThePriceDelayed("64,000.00");

    await alice.guesses("Up");

    await alice.should.haveTheGuessRefusedAs("priceDelayed");
    await alice.should.haveNoOpenGuess();
  },
);

test(
  'When a Guess is refused as rateLimited, the Player sees "Too many guesses from your network. Try again in a minute."',
  // UI only: the message is part of the page.
  { tag: "@ui" },
  async ({ alice, neighbours, priceSource }) => {
    await priceSource.sends("64,000.00");
    await alice.opensTheGame();

    for (const neighbour of await neighbours.arrive(10)) {
      await neighbour.opensTheGame();
      await neighbour.guesses("Up");
      await neighbour.should.haveAnOpenGuess("Up", "64,000.00");
    }

    await alice.guesses("Up");

    await alice.should.haveTheGuessRefusedAs("rateLimited");
    await alice.should.haveNoOpenGuess();
  },
);

test(
  "Up, Down, the Score, the price and the Open guess are reachable by role and accessible name, and Up and Down work from the keyboard with a visible Horn focus ring",
  // UI only: roles, names and the keyboard are part of the page.
  { tag: "@ui" },
  async ({ alice, priceSource }) => {
    await priceSource.sends("64,000.00");
    await alice.opensTheGame();
    await alice.should.beAbleToGuess();

    await alice.movesTheFocusTo("Down");

    await alice.should.seeAHornFocusRingOn("Down");

    await alice.movesTheFocusTo("Up");

    await alice.should.seeAHornFocusRingOn("Up");

    await alice.pressesEnter();

    await alice.should.haveAnOpenGuess("Up", "64,000.00");
    await alice.should.findEverythingByRoleAndName();
  },
);

test(
  "An IP that places more than 10 Guesses a minute, after a burst of 10, is refused with rateLimited",
  // API only: the page shows the refusal as a message, a case of its own.
  { tag: "@api" },
  async ({ alice, neighbours, priceSource }) => {
    await priceSource.sends("64,000.00");

    for (const neighbour of await neighbours.arrive(10)) {
      await neighbour.opensTheGame();
      await neighbour.guesses("Up");
      await neighbour.should.haveAnOpenGuess("Up", "64,000.00");
    }

    await alice.opensTheGame();
    await alice.guesses("Up");

    await alice.should.haveTheGuessRefusedAs("rateLimited");
    await alice.should.haveNoOpenGuess();
  },
);

test(
  "An IP that receives more than 30 new identity cookies an hour, after a burst of 30, gets the page without a cookie and is refused with rateLimited by the game",
  // API only: the page does not show why the game turned it away.
  { tag: "@api" },
  async ({ alice, neighbours }) => {
    await alice.opensTheGame();

    for (const neighbour of await neighbours.arrive(29)) {
      await neighbour.opensTheGame();
      await neighbour.should.beKnownByThisBrowser();
    }

    const [late] = await neighbours.arrive(1);

    if (late === undefined) {
      throw new Error("a neighbour should arrive");
    }

    await late.opensThePage();

    await late.should.notBeKnownByThisBrowser();
    await late.should.beTurnedAwayAs("rateLimited");
  },
);
