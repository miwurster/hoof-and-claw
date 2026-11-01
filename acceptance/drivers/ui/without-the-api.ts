// What only the HTTP API can do, which the UI driver refuses: requests the page never sends, holding the stream, and its keepalive.

/** The part of a Player's channel that needs the HTTP API itself. */
export class WithoutTheApi {
  guessesWithAPriceAndTimeOfTheirOwn(): Promise<void> {
    return Promise.reject(new Error("the page sends only a Direction; only the API can carry a price or a time"));
  }

  guessesADirectionThatIsNeitherUpNorDown(): Promise<void> {
    return Promise.reject(new Error("the page has only Up and Down"));
  }

  guessesWithABodyThatIsNotJson(): Promise<void> {
    return Promise.reject(new Error("the page always sends JSON"));
  }

  guessesWithoutSayingTheBodyIsJson(): Promise<void> {
    return Promise.reject(new Error("the page always sends JSON"));
  }

  triesToOpenTheGame(): Promise<null> {
    return Promise.reject(new Error("a page holds one stream; only the API can open more"));
  }

  holdsTheStream(): Promise<number> {
    return Promise.reject(new Error("the page reconnects its stream by itself; hold the stream through the API"));
  }

  expectsAKeepalive(): Promise<void> {
    return Promise.reject(new Error("a page shows no keepalive; only the API sees it arrive"));
  }

  expectsTurnedAway(): Promise<void> {
    return Promise.reject(new Error("the page does not show why the game's API turned it away"));
  }
}
