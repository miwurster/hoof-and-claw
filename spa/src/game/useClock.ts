import { useEffect, useState } from "react";

/** How often the countdown reads the page's clock. */
const TICK_MS = 250;

/** The page's own clock, which the countdown and the reconnect delays run on; never compared with the server's time. */
export function pageClock(): number {
  return performance.now();
}

/**
 * The page's clock, read a few times a second while `running`, for the countdown of the Open guess.
 * The default `now` is one function for every render: a fresh closure per render would re-run the effect on each
 * render, and its `setTime` would render again, without end.
 */
export function useClock(running: boolean, now: () => number = pageClock): number {
  const [time, setTime] = useState(now);

  useEffect(() => {
    const timer = running
      ? setInterval(() => {
          setTime(now());
        }, TICK_MS)
      : undefined;

    setTime(now());

    return () => {
      clearInterval(timer);
    };
  }, [running, now]);

  return time;
}
