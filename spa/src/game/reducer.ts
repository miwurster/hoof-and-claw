import type { ServerEvent } from "contract";

/** Everything the game shows, built from the stream's snapshot and kept up to date by its events. */
export type GameState = {
  readonly price: number | null;
  readonly delayed: boolean;
  readonly score: number;
};

/** Before the stream opens, the page knows no price and treats it as delayed. */
export const initialState: GameState = { price: null, delayed: true, score: 0 };

export function reduce(state: GameState, event: ServerEvent): GameState {
  if (event.type === "snapshot") {
    return { price: event.price, delayed: event.delayed, score: event.score };
  }

  return { ...state, price: event.price, delayed: event.delayed };
}

/** A Guess needs a live price to start from. */
export function canGuess(state: GameState): boolean {
  return !state.delayed && state.price !== null;
}
