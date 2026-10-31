import { useCallback, useSyncExternalStore } from "react";

/** The part of `MediaQueryList` the hook uses. */
export type MotionQuery = {
  readonly matches: boolean;
  readonly addEventListener: (type: "change", listener: () => void) => void;
  readonly removeEventListener: (type: "change", listener: () => void) => void;
};

/** The browser's `prefers-reduced-motion` query. */
function openQuery(): MotionQuery {
  return window.matchMedia("(prefers-reduced-motion: reduce)");
}

/** Whether the Player prefers reduced motion, following the preference as it changes. */
export function useReducedMotion(open: () => MotionQuery = openQuery): boolean {
  const subscribe = useCallback(
    (changed: () => void) => {
      const query = open();

      query.addEventListener("change", changed);

      return () => {
        query.removeEventListener("change", changed);
      };
    },
    [open],
  );

  return useSyncExternalStore(subscribe, () => open().matches);
}
