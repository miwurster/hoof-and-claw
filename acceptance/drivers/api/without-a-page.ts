// What only the page has, which the API driver refuses: buttons, the keyboard, colours and a screen reader.

/** The part of a Player's channel that needs the page in a browser. */
export class WithoutAPage {
  startsToGuess(): Promise<void> {
    return Promise.reject(new Error("the API has no buttons to disable while a Guess is being placed"));
  }

  receivesTheAnswer(): Promise<void> {
    return Promise.reject(new Error("the API has no buttons to disable while a Guess is being placed"));
  }

  movesTheFocusTo(): Promise<void> {
    return Promise.reject(new Error("the API has no keyboard focus"));
  }

  pressesEnter(): Promise<void> {
    return Promise.reject(new Error("the API has no keyboard"));
  }

  expectsDifference(): Promise<void> {
    return Promise.reject(new Error("the API carries no difference from the Start price; only the page shows it"));
  }

  expectsWaitingForThePriceToMove(): Promise<void> {
    return Promise.reject(new Error("the API has no waiting label; only the page shows it"));
  }

  expectsOutcomeAnnounced(): Promise<void> {
    return Promise.reject(new Error("the API has no screen reader"));
  }

  expectsAnnouncement(): Promise<void> {
    return Promise.reject(new Error("the API has no screen reader"));
  }

  expectsWhereTheScoreLives(): Promise<void> {
    return Promise.reject(new Error("the API has no page"));
  }

  expectsEverythingByRoleAndName(): Promise<void> {
    return Promise.reject(new Error("the API has no roles or accessible names"));
  }

  expectsHornFocusRing(): Promise<void> {
    return Promise.reject(new Error("the API has no focus ring"));
  }
}
