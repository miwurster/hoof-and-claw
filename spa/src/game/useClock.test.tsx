import { act, renderHook } from "@testing-library/react";
import { expect, test, vi } from "vitest";

import { useClock } from "./useClock";

test("with the default clock, a render does not cause another", () => {
  let renders = 0;

  renderHook(() => {
    renders += 1;

    // A clock that changed on every render would render without end; stop that run before it runs out of memory.
    if (renders > 10) {
      throw new Error(`rendered ${renders} times`);
    }

    return useClock(true);
  });

  expect(renders).toBeLessThanOrEqual(2);
});

test("while running, the clock is read a few times a second", () => {
  vi.useFakeTimers();
  let now = 1_000;

  const { result } = renderHook(() => useClock(true, () => now));

  now = 1_250;
  act(() => {
    vi.advanceTimersByTime(250);
  });

  expect(result.current).toBe(1_250);
  vi.useRealTimers();
});

test("while not running, the clock is read once and then stands still", () => {
  vi.useFakeTimers();
  let now = 1_000;

  const { result } = renderHook(() => useClock(false, () => now));

  now = 5_000;
  act(() => {
    vi.advanceTimersByTime(1_000);
  });

  expect(result.current).toBe(1_000);
  vi.useRealTimers();
});
