import { act, renderHook } from "@testing-library/react";
import type { ServerEvent } from "contract";
import { expect, test } from "vitest";

import type { Stream } from "./useGame";
import { useGame } from "./useGame";

class FakeStream implements Stream {
  closed = false;
  private readonly listeners: ((message: MessageEvent<string>) => void)[] = [];

  constructor(readonly url: string) {}

  addEventListener(_type: "message", listener: (message: MessageEvent<string>) => void): void {
    this.listeners.push(listener);
  }

  close(): void {
    this.closed = true;
  }

  emit(event: ServerEvent): void {
    const message = new MessageEvent("message", { data: JSON.stringify(event) });

    for (const listener of this.listeners) {
      listener(message);
    }
  }
}

function fakeStreams() {
  const opened: FakeStream[] = [];

  return {
    opened,
    open: (url: string) => {
      const stream = new FakeStream(url);

      opened.push(stream);

      return stream;
    },
  };
}

test("the hook opens the game's stream and applies its events", () => {
  const streams = fakeStreams();
  const { result } = renderHook(() => useGame(streams.open));
  const stream = streams.opened.at(-1);

  expect(stream?.url).toBe("/api/events");

  act(() => {
    stream?.emit({
      type: "snapshot",
      build: "abc",
      serverTime: "2027-01-01T00:00:00Z",
      price: 6_400_000,
      delayed: false,
      score: 1,
      openGuess: null,
      lastResolved: null,
    });
    stream?.emit({ type: "price", price: 6_400_001, delayed: false });
  });

  expect(result.current).toEqual({ price: 6_400_001, delayed: false, score: 1 });
});

test("the hook closes its stream when the page leaves the game", () => {
  const streams = fakeStreams();
  const { unmount } = renderHook(() => useGame(streams.open));

  unmount();

  expect(streams.opened.every((stream) => stream.closed)).toBe(true);
});
