// The messages the page shows the Player.

/** The message the page shows for a refused Guess; for the other refusals the page already shows why. */
export function refusalMessage(refusal: string): string {
  switch (refusal) {
    case "rateLimited":
      return "Too many guesses from your network. Try again in a minute.";
    case "internal":
      return "Couldn't place your guess. Try again.";
    default:
      throw new Error(`the page shows no message for a Guess refused as ${refusal}`);
  }
}
