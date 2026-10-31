import type { ServerEvent } from "contract";
import { useEffect, useReducer } from "react";

import { initialState, reduce } from "./reducer";
import type { GameState } from "./reducer";

/** The part of `EventSource` the hook uses. */
export type Stream = {
  addEventListener: (type: "message", listener: (message: MessageEvent<string>) => void) => void;
  close: () => void;
};

function openEventSource(url: string): Stream {
  return new EventSource(url);
}

/** Decodes an event of the stream; the one place the SPA trusts the server's JSON. */
function decode(data: string): ServerEvent {
  // SAFETY: the SPA and the server come from the same commit and the same origin (ADR 0014).
  // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- JSON.parse returns any.
  return JSON.parse(data) as ServerEvent;
}

/** The game's state, fed by the server's stream for as long as the page shows the game. */
export function useGame(openStream: (url: string) => Stream = openEventSource): GameState {
  const [state, dispatch] = useReducer(reduce, initialState);

  useEffect(() => {
    const stream = openStream("/api/events");

    stream.addEventListener("message", (message) => {
      dispatch(decode(message.data));
    });

    return () => {
      stream.close();
    };
  }, [openStream]);

  return state;
}
