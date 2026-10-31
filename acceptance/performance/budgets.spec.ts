// The performance budgets under 200 Players (ADR 0015); the budgets are the harness's configuration.
import { seconds, test } from "../dsl";

const PLAYERS = 200;

/** Ten changed prices, so the p95 of the price fan-out has more than one sample. */
const PRICES = [
  "64,000.01",
  "64,000.02",
  "64,000.03",
  "64,000.04",
  "64,000.05",
  "64,000.06",
  "64,000.07",
  "64,000.08",
  "64,000.09",
  "64,000.10",
];

test("With 200 Players, a price reaches the last of 200 streams within p95 250 ms and at most 1 s", async ({
  players,
  priceSource,
}) => {
  const crowd = await players(PLAYERS);

  await priceSource.sends("64,000.00");
  await crowd.openTheGame();

  for (const price of PRICES) {
    await crowd.receiveThePriceSentBy(priceSource, price);
  }

  await crowd.should.receiveEachPriceOnTheLastStreamWithinBudget();
});

test("With 200 Players, placing a Guess takes p95 under 100 ms", async ({ players, priceSource }) => {
  const crowd = await players(PLAYERS);

  await priceSource.sends("64,000.00");
  await crowd.openTheGame();
  await crowd.guess("Up");

  await crowd.should.placeTheirGuessesWithinBudget();
});

test("200 Guesses due on one price all resolve, until the last Guess resolved event, within 2 s", async ({
  players,
  priceSource,
  clock,
}) => {
  const crowd = await players(PLAYERS);

  await priceSource.sends("64,000.00");
  await crowd.openTheGame();
  await crowd.guess("Up");
  await clock.advances(seconds(60));
  await crowd.seeTheirGuessesResolvedByThePriceSentBy(priceSource, "64,000.01");

  await crowd.should.seeEveryGuessResolvedWithinBudget();
});

test("With 200 Players, a stream's first event arrives within p95 200 ms of opening it", async ({
  players,
  priceSource,
}) => {
  const crowd = await players(PLAYERS);

  await priceSource.sends("64,000.00");
  await crowd.openTheGame();

  await crowd.should.receiveTheFirstEventWithinBudget();
});

test("The server's resident memory with 200 streams open stays under 128 MB, with zero errors and zero streams closed by the server", async ({
  players,
  priceSource,
  clock,
}) => {
  const crowd = await players(PLAYERS);

  await priceSource.sends("64,000.00");
  await crowd.openTheGame();
  await crowd.guess("Down");
  await clock.advances(seconds(60));
  await crowd.seeTheirGuessesResolvedByThePriceSentBy(priceSource, "63,999.99");

  await crowd.should.leaveTheServerWithinItsMemoryBudget();
  await crowd.should.haveNoStreamClosedByTheServer();
});
