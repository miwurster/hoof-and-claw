import type { ErrorCode, OpenGuess, ServerEvent } from "contract";

/** The Open guess, and when it falls due on the page's own clock: its remaining time after it arrived. */
export type HeldGuess = {
  readonly guess: OpenGuess;
  readonly dueAt: number;
};

/** The one line under Up and Down after a Guess could not be placed. */
export type Message = "rateLimited" | "failed";

/** Everything the game shows, built from the stream's snapshot and kept up to date by its events and the Player. */
export type GameState = {
  readonly price: number | null;
  readonly delayed: boolean;
  readonly score: number;
  readonly openGuess: HeldGuess | null;
  /** From the press until the answer or the Guess placed event arrives. */
  readonly placing: boolean;
  readonly message: Message | null;
};

/** What happens to the game: a server event as it arrives, or a step of placing a Guess. `at` is the page's clock. */
export type Action =
  | { readonly type: "received"; readonly event: ServerEvent; readonly at: number }
  | { readonly type: "guessing" }
  | { readonly type: "guessAnswered"; readonly guess: OpenGuess; readonly at: number }
  | { readonly type: "guessFailed"; readonly error: ErrorCode | "network" };

/** Before the stream opens, the page knows no price and treats it as delayed. */
export const initialState: GameState = {
  price: null,
  delayed: true,
  score: 0,
  openGuess: null,
  placing: false,
  message: null,
};

/** Holds a Guess by its ID, so the answer and the Guess placed event may arrive in either order (ADR 0014). */
function hold(current: HeldGuess | null, guess: OpenGuess, at: number): HeldGuess {
  return current?.guess.id === guess.id ? current : { guess, dueAt: at + guess.remainingMs };
}

function messageFor(error: ErrorCode | "network"): Message | null {
  if (error === "rateLimited") {
    return "rateLimited";
  }

  // The stream already shows the Open guess or Price delayed.
  return error === "openGuessExists" || error === "priceDelayed" ? null : "failed";
}

function receive(state: GameState, event: ServerEvent, at: number): GameState {
  if (event.type === "snapshot") {
    return {
      ...state,
      price: event.price,
      delayed: event.delayed,
      score: event.score,
      openGuess: event.openGuess === null ? null : hold(state.openGuess, event.openGuess, at),
    };
  }

  if (event.type === "price") {
    return { ...state, price: event.price, delayed: event.delayed };
  }

  return { ...state, openGuess: hold(state.openGuess, event.guess, at), placing: false };
}

export function reduce(state: GameState, action: Action): GameState {
  if (action.type === "received") {
    return receive(state, action.event, action.at);
  }

  if (action.type === "guessing") {
    return { ...state, placing: true, message: null };
  }

  if (action.type === "guessAnswered") {
    return { ...state, openGuess: hold(state.openGuess, action.guess, action.at), placing: false };
  }

  return { ...state, placing: false, message: messageFor(action.error) };
}

/** A Guess needs a live price to start from, and the Player may hold only one at a time. */
export function canGuess(state: GameState): boolean {
  return !state.delayed && state.price !== null && !state.placing && state.openGuess === null;
}
