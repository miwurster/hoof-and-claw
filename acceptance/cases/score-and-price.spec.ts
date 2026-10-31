import { test } from "../dsl";

test("A Player sees the latest price, and a changed price appears without a reload", async ({ alice, priceSource }) => {
  await priceSource.sends("64,000.00");
  await alice.opensTheGame();

  await alice.should.seeThePrice("64,000.00");

  await priceSource.sends("64,000.01");

  await alice.should.seeThePrice("64,000.01");
});

test(
  "The price is formatted in US dollars with cents, such as $67,412.18",
  // UI only: the API carries prices in cents, and only the page formats dollars.
  { tag: "@ui" },
  async ({ alice, priceSource }) => {
    await priceSource.sends("67,412.18");
    await alice.opensTheGame();

    await alice.should.seeThePrice("67,412.18");
  },
);

test("A Player sees their Score on every screen state", async ({ alice, priceSource }) => {
  await alice.opensTheGame();

  await alice.should.seeThePriceDelayed(null);
  await alice.should.haveScore(0);

  await priceSource.sends("64,000.00");

  await alice.should.seeThePrice("64,000.00");
  await alice.should.haveScore(0);

  await priceSource.losesTheConnection();

  await alice.should.seeThePriceDelayed("64,000.00");
  await alice.should.haveScore(0);
});

test("A new Player starts with Score 0 and no Open guess", async ({ alice, priceSource }) => {
  await priceSource.sends("64,000.00");
  await alice.opensTheGame();

  await alice.should.haveScore(0);
  await alice.should.haveNoOpenGuess();
});

test("A visitor who has not placed a Guess gets an identity cookie and reads as Score 0 with no Open guess", async ({
  alice,
}) => {
  await alice.opensTheGame();

  await alice.should.beKnownByThisBrowser();
  await alice.should.haveScore(0);
  await alice.should.haveNoOpenGuess();
});

test("While the price is delayed, a Player sees the last price labelled Price delayed, with Up and Down disabled", async ({
  alice,
  priceSource,
}) => {
  await priceSource.sends("64,000.00");
  await alice.opensTheGame();
  await alice.should.seeThePrice("64,000.00");

  await priceSource.losesTheConnection();

  await alice.should.seeThePriceDelayed("64,000.00");
  await alice.should.notBeAbleToGuess();
});

test("Before the server has received any price, a Player sees — labelled Price delayed and cannot place a Guess", async ({
  alice,
}) => {
  await alice.opensTheGame();

  await alice.should.seeThePriceDelayed(null);
  await alice.should.notBeAbleToGuess();
});

test("When the price recovers, the label disappears and Up and Down are enabled again", async ({
  alice,
  priceSource,
}) => {
  await priceSource.sends("64,000.00");
  await alice.opensTheGame();
  await priceSource.losesTheConnection();
  await alice.should.seeThePriceDelayed("64,000.00");

  await priceSource.restoresTheConnection();

  await alice.should.seeThePrice("64,000.00");
  await alice.should.beAbleToGuess();
});

test(
  "A screen reader announces changes between live and Price delayed politely",
  // UI only: a screen reader reads the page.
  { tag: "@ui" },
  async ({ alice, priceSource }) => {
    await priceSource.sends("64,000.00");
    await alice.opensTheGame();
    await alice.should.hearThePriceAnnouncedAs("Price live");

    await priceSource.losesTheConnection();

    await alice.should.hearThePriceAnnouncedAs("Price delayed");

    await priceSource.restoresTheConnection();

    await alice.should.hearThePriceAnnouncedAs("Price live");
  },
);

test(
  'The page always shows "Your score lives in this browser." beneath the rules line',
  // UI only: the line is part of the page.
  { tag: "@ui" },
  async ({ alice }) => {
    await alice.opensTheGame();

    await alice.should.beToldWhereTheScoreLives();
  },
);
