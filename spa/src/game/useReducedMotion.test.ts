import { act, renderHook } from "@testing-library/react";
import { expect, test } from "vitest";

import { useReducedMotion } from "./useReducedMotion";
import type { MotionQuery } from "./useReducedMotion";

/** A media query whose answer the test changes. */
function query(matches: boolean) {
  const listeners = new Set<() => void>();

  const media = {
    matches,
    addEventListener: (_type: "change", listener: () => void) => {
      listeners.add(listener);
    },
    removeEventListener: (_type: "change", listener: () => void) => {
      listeners.delete(listener);
    },
  };

  return {
    open: (): MotionQuery => media,
    change: (next: boolean) => {
      media.matches = next;

      for (const listener of listeners) {
        listener();
      }
    },
  };
}

test("a Player who prefers reduced motion gets no animation", () => {
  const { open } = query(true);

  const { result } = renderHook(() => useReducedMotion(open));

  expect(result.current).toBe(true);
});

test("the preference is followed when it changes", () => {
  const { open, change } = query(false);
  const { result } = renderHook(() => useReducedMotion(open));

  expect(result.current).toBe(false);

  act(() => {
    change(true);
  });

  expect(result.current).toBe(true);
});
