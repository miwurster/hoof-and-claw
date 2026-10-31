import { act, renderHook } from "@testing-library/react";
import type { OpenGuess, ServerEvent } from "contract";
import { expect, test } from "vitest";

import type { Send, Stream } from "./useGame";
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

/** A server that answers every request with one status and body, and records what it was sent. */
function fakeServer(status: number, body: string) {
  const requests: { url: string; method: string | undefined; contentType: string | null; body: unknown }[] = [];

  const send: Send = (url, init) => {
    requests.push({
      url,
      method: init.method,
      contentType: new Headers(init.headers).get("content-type"),
      body: init.body,
    });

    return Promise.resolve(new Response(body, { status }));
  };

  return { requests, send };
}

/** A server the page cannot reach. */
const unreachable: Send = () => Promise.reject(new TypeError("Failed to fetch"));

const snapshot: ServerEvent = {
  type: "snapshot",
  build: "abc",
  serverTime: "2027-01-01T00:00:00Z",
  price: 6_400_000,
  delayed: false,
  score: 1,
  openGuess: null,
  lastResolved: null,
};

const guess: OpenGuess = { id: "guess-1", direction: "down", startPrice: 6_400_000, remainingMs: 60_000 };

test("the hook opens the game's stream and applies its events", () => {
  const streams = fakeStreams();
  const { result } = renderHook(() => useGame(streams.open));
  const stream = streams.opened.at(-1);

  expect(stream?.url).toBe("/api/events");

  act(() => {
    stream?.emit(snapshot);
    stream?.emit({ type: "price", price: 6_400_001, delayed: false });
  });

  expect(result.current.state).toMatchObject({ price: 6_400_001, delayed: false, score: 1 });
});

test("the hook closes its stream when the page leaves the game", () => {
  const streams = fakeStreams();
  const { unmount } = renderHook(() => useGame(streams.open));

  unmount();

  expect(streams.opened.every((stream) => stream.closed)).toBe(true);
});

test("placing a Guess sends only its Direction as JSON, and the answer becomes the Open guess", async () => {
  const streams = fakeStreams();
  const server = fakeServer(201, JSON.stringify(guess));
  const { result } = renderHook(() => useGame(streams.open, server.send, () => 5_000));

  act(() => {
    streams.opened.at(-1)?.emit(snapshot);
  });
  await act(async () => {
    await result.current.place("down");
  });

  expect(server.requests).toEqual([
    { url: "/api/guesses", method: "POST", contentType: "application/json", body: '{"direction":"down"}' },
  ]);
  expect(result.current.state.openGuess).toEqual({ guess, dueAt: 65_000 });
  expect(result.current.state.placing).toBe(false);
});

test("Up and Down stay disabled from the press until the answer arrives", async () => {
  const streams = fakeStreams();
  let answer: ((response: Response) => void) | undefined;

  const send: Send = () =>
    new Promise((resolve) => {
      answer = resolve;
    });

  const { result } = renderHook(() => useGame(streams.open, send));

  act(() => {
    streams.opened.at(-1)?.emit(snapshot);
  });

  let placed: Promise<void> = Promise.resolve();

  act(() => {
    placed = result.current.place("up");
  });

  expect(result.current.state.placing).toBe(true);

  await act(async () => {
    answer?.(new Response(JSON.stringify(guess), { status: 201 }));
    await placed;
  });

  expect(result.current.state.placing).toBe(false);
});

test("a refused Guess reports the server's error code", async () => {
  const streams = fakeStreams();
  const server = fakeServer(429, JSON.stringify({ error: "rateLimited" }));
  const { result } = renderHook(() => useGame(streams.open, server.send));

  await act(async () => {
    await result.current.place("up");
  });

  expect(result.current.state.message).toBe("rateLimited");
});

test("a Guess that cannot reach the server reports a network error", async () => {
  const streams = fakeStreams();
  const { result } = renderHook(() => useGame(streams.open, unreachable));

  await act(async () => {
    await result.current.place("up");
  });

  expect(result.current.state.message).toBe("failed");
  expect(result.current.state.placing).toBe(false);
});

test("an answer that is no error body, such as a proxy's page, counts as a failure", async () => {
  const streams = fakeStreams();
  const server = fakeServer(502, "<html>Bad Gateway</html>");
  const { result } = renderHook(() => useGame(streams.open, server.send));

  await act(async () => {
    await result.current.place("up");
  });

  expect(result.current.state.message).toBe("failed");
});
