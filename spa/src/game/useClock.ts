import { useEffect, useState } from "react";

/** How often the countdown reads the page's clock. */
const TICK_MS = 250;

/** The page's own clock, read a few times a second while `running`, for the countdown of the Open guess. */
export function useClock(running: boolean, now: () => number = () => performance.now()): number {
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
