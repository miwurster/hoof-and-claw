import { act, renderHook } from "@testing-library/react";
import type { OpenGuess } from "contract";
import { expect, test } from "vitest";

import { fakeBrowser, snapshot } from "./fakeBrowser";
import type { Send } from "./useGame";
import { useGame } from "./useGame";

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

const guess: OpenGuess = { id: "guess-1", direction: "down", startPrice: 6_400_000, remainingMs: 60_000 };

/** The page clock of the Guess tests, at 5 s. */
const atFiveSeconds = () => 5_000;

test("placing a Guess sends only its Direction as JSON, and the answer becomes the Open guess", async () => {
  const page = fakeBrowser();
  const server = fakeServer(201, JSON.stringify(guess));
  const { result } = renderHook(() => useGame(page.browser, server.send, atFiveSeconds));

  act(() => {
    page.latest().emit(snapshot());
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
  const page = fakeBrowser();
  let answer: ((response: Response) => void) | undefined;

  const send: Send = () =>
    new Promise((resolve) => {
      answer = resolve;
    });

  const { result } = renderHook(() => useGame(page.browser, send));

  act(() => {
    page.latest().emit(snapshot());
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
  const page = fakeBrowser();
  const server = fakeServer(429, JSON.stringify({ error: "rateLimited" }));
  const { result } = renderHook(() => useGame(page.browser, server.send));

  await act(async () => {
    await result.current.place("up");
  });

  expect(result.current.state.message).toBe("rateLimited");
});

test("a Guess that cannot reach the server reports a network error", async () => {
  const page = fakeBrowser();
  const { result } = renderHook(() => useGame(page.browser, unreachable));

  await act(async () => {
    await result.current.place("up");
  });

  expect(result.current.state.message).toBe("failed");
  expect(result.current.state.placing).toBe(false);
});

test("an answer that is no error body, such as a proxy's page, counts as a failure", async () => {
  const page = fakeBrowser();
  const server = fakeServer(502, "<html>Bad Gateway</html>");
  const { result } = renderHook(() => useGame(page.browser, server.send));

  await act(async () => {
    await result.current.place("up");
  });

  expect(result.current.state.message).toBe("failed");
});
