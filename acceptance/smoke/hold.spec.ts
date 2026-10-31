// `pnpm smoke:hold`, run by hand once after the first Release (ADR 0003, ADR 0013).
// If Render's proxy cuts the stream, the page reconnects with a fresh snapshot, and the heartbeat interval is adjusted.
import { holdMinutes, test } from "../dsl/deployed";

test("A stream to the deployed service stays open for two hours without the server closing it", async ({ alice }) => {
  test.setTimeout((holdMinutes + 5) * 60_000);

  await alice.opensTheGame();

  await alice.should.keepTheStreamOpenFor(holdMinutes);
});
