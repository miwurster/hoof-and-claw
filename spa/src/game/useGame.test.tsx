import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, expect, test, vi } from "vitest";

import { fakeBrowser, snapshot } from "./fakeBrowser";
import { useGame } from "./useGame";

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
});

test("the hook opens the game's stream and applies its events", () => {
  const page = fakeBrowser();
  const { result } = renderHook(() => useGame(page.browser));

  expect(page.latest().url).toBe("/api/events");

  act(() => {
    page.latest().emit(snapshot());
    page.latest().emit({ type: "price", price: 6_400_001, delayed: false });
  });

  expect(result.current.state).toMatchObject({ price: 6_400_001, delayed: false, score: 1 });
});

test("the hook closes its stream when the page leaves the game", () => {
  const page = fakeBrowser();
  const { unmount } = renderHook(() => useGame(page.browser));

  unmount();

  expect(page.opened.every((stream) => stream.closed)).toBe(true);
});

test("a lost stream shows the last price as delayed and reconnects after a delay", () => {
  const page = fakeBrowser();
  const { result } = renderHook(() => useGame(page.browser));

  act(() => {
    page.latest().emit(snapshot());
    page.latest().fail();
  });

  expect(result.current.state).toMatchObject({ price: 6_400_000, delayed: true, score: 1 });
  expect(page.opened).toHaveLength(1);
  expect(page.latest().closed).toBe(true);

  act(() => {
    vi.advanceTimersByTime(1_000);
  });

  expect(page.opened).toHaveLength(2);

  act(() => {
    page.latest().emit({ ...snapshot(), price: 6_500_000, score: 2 });
  });

  expect(result.current.state).toMatchObject({ price: 6_500_000, delayed: false, score: 2 });
});

test("each reconnect without a snapshot waits a random delay that grows, and a snapshot starts over", () => {
  const page = fakeBrowser(0);

  const opensAfter = (milliseconds: number) => {
    const before = page.opened.length;

    act(() => {
      page.latest().fail();
      vi.advanceTimersByTime(milliseconds - 1);
    });
    expect(page.opened, `no reconnect before ${String(milliseconds)} ms`).toHaveLength(before);
    act(() => {
      vi.advanceTimersByTime(1);
    });
    expect(page.opened, `a reconnect after ${String(milliseconds)} ms`).toHaveLength(before + 1);
  };

  renderHook(() => useGame(page.browser));

  opensAfter(500);
  opensAfter(1_000);
  opensAfter(2_000);
  opensAfter(4_000);
  opensAfter(8_000);
  opensAfter(15_000);
  opensAfter(15_000);

  act(() => {
    page.latest().emit(snapshot());
  });

  opensAfter(500);
});

test("the random part of the delay spreads reconnects up to twice the shortest delay", () => {
  const page = fakeBrowser(0.999);

  renderHook(() => useGame(page.browser));

  act(() => {
    page.latest().fail();
    vi.advanceTimersByTime(998);
  });
  expect(page.opened).toHaveLength(1);

  act(() => {
    vi.advanceTimersByTime(1);
  });
  expect(page.opened).toHaveLength(2);
});

test("a stream without any data for 45 seconds is dead: the hook shows the price delayed and reconnects", () => {
  const page = fakeBrowser();
  const { result } = renderHook(() => useGame(page.browser));

  act(() => {
    page.latest().emit(snapshot());
    vi.advanceTimersByTime(44_999);
    page.latest().emit({ type: "price", price: 6_400_001, delayed: false });
    vi.advanceTimersByTime(44_999);
  });

  expect(page.latest().closed).toBe(false);
  expect(result.current.state.delayed).toBe(false);

  act(() => {
    vi.advanceTimersByTime(1);
  });

  expect(page.latest().closed).toBe(true);
  expect(result.current.state).toMatchObject({ price: 6_400_001, delayed: true, score: 1 });

  act(() => {
    vi.advanceTimersByTime(500);
  });

  expect(page.opened).toHaveLength(2);
  expect(page.latest().closed).toBe(false);
});

test("a stream that receives only keepalives for 45 seconds and more stays open", () => {
  const page = fakeBrowser();
  const { result } = renderHook(() => useGame(page.browser));

  act(() => {
    page.latest().emit(snapshot());

    for (let second = 15; second <= 90; second += 15) {
      vi.advanceTimersByTime(15_000);
      page.latest().keepalive();
    }
  });

  expect(page.opened).toHaveLength(1);
  expect(page.latest().closed).toBe(false);
  expect(result.current.state.delayed).toBe(false);
});

test("the hook closes the old stream before it opens a new one", () => {
  const page = fakeBrowser();

  renderHook(() => useGame(page.browser));

  act(() => {
    page.latest().fail();
    vi.advanceTimersByTime(30_000);
    vi.advanceTimersByTime(45_000);
    vi.advanceTimersByTime(30_000);
  });

  expect(page.opened.length).toBeGreaterThanOrEqual(3);
  expect(page.openedWhileAnotherWasOpen).toEqual([]);
});

test("a closed stream changes nothing, even when it still delivers", () => {
  const page = fakeBrowser();
  const { result } = renderHook(() => useGame(page.browser));
  const old = page.latest();

  act(() => {
    old.emit(snapshot());
    old.fail();
    vi.advanceTimersByTime(500);
    page.latest().emit({ ...snapshot(), price: 6_500_000 });
    old.emit({ type: "price", price: 1, delayed: false });
    old.fail();
  });

  expect(result.current.state).toMatchObject({ price: 6_500_000, delayed: false, score: 1 });
  expect(page.opened).toHaveLength(2);
});

test("when a snapshot's build differs from the page's own build, the page reloads", () => {
  const page = fakeBrowser();

  renderHook(() => useGame(page.browser));

  act(() => {
    page.latest().emit(snapshot("abc"));
    page.latest().fail();
    vi.advanceTimersByTime(500);
    page.latest().emit(snapshot("abc"));
  });

  expect(page.reloads()).toBe(0);

  act(() => {
    page.latest().fail();
    vi.advanceTimersByTime(500);
    page.latest().emit(snapshot("def"));
  });

  expect(page.reloads()).toBe(1);
  expect(page.latest().closed).toBe(true);
});
