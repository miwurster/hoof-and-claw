// The browser and stream a test of the game hook stands in for the page's.
import type { ServerEvent } from "contract";

import type { Browser, Stream } from "./useGame";

type Listener = (message: MessageEvent<string>) => void;

export class FakeStream implements Stream {
  closed = false;
  private readonly messages: Listener[] = [];
  private readonly errors: (() => void)[] = [];

  constructor(readonly url: string) {}

  addEventListener(type: "message" | "error", listener: Listener): void {
    if (type === "message") {
      this.messages.push(listener);
    } else {
      this.errors.push(() => {
        listener(new MessageEvent("error"));
      });
    }
  }

  close(): void {
    this.closed = true;
  }

  emit(event: ServerEvent): void {
    const message = new MessageEvent("message", { data: JSON.stringify(event) });

    for (const listener of this.messages) {
      listener(message);
    }
  }

  fail(): void {
    for (const listener of this.errors) {
      listener();
    }
  }
}

/** A browser whose streams, random numbers and reloads the test controls. */
export function fakeBrowser(random = 0) {
  const opened: FakeStream[] = [];
  const openedWhileAnotherWasOpen: string[] = [];
  let reloads = 0;

  const browser: Browser = {
    openStream: (url: string) => {
      if (opened.some((stream) => !stream.closed)) {
        openedWhileAnotherWasOpen.push(url);
      }

      const stream = new FakeStream(url);

      opened.push(stream);

      return stream;
    },
    random: () => random,
    reload: () => {
      reloads += 1;
    },
  };

  return {
    browser,
    opened,
    openedWhileAnotherWasOpen,
    reloads: () => reloads,
    latest: (): FakeStream => {
      const stream = opened.at(-1);

      if (stream === undefined) {
        throw new Error("no stream was opened");
      }

      return stream;
    },
  };
}

export function snapshot(build = "abc"): Extract<ServerEvent, { type: "snapshot" }> {
  return {
    type: "snapshot",
    build,
    serverTime: "2027-01-01T00:00:00Z",
    price: 6_400_000,
    delayed: false,
    score: 1,
    openGuess: null,
    lastResolved: null,
  };
}
