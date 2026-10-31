import { Directions } from "../components/Directions";
import { GuessCard } from "../components/GuessCard";
import { GuessMessage } from "../components/GuessMessage";
import { Masthead } from "../components/Masthead";
import { Page } from "../components/Page";
import { Price } from "../components/Price";
import { Rules } from "../components/Rules";
import { useClock } from "../game/useClock";
import { useGame } from "../game/useGame";
import { useReducedMotion } from "../game/useReducedMotion";

/** The container of the route `/`: wires the game to the presentational components. */
export function GameRoute() {
  const { state, canGuess, place } = useGame();
  const now = useClock(state.openGuess !== null);
  const reducedMotion = useReducedMotion();
  const held = state.openGuess;

  return (
    <Page>
      <Masthead score={state.score} />
      <Price price={state.price} delayed={state.delayed} />
      <Directions
        enabled={canGuess}
        onGuess={(direction) => {
          void place(direction);
        }}
      />
      <GuessMessage message={state.message} />
      <Rules />
      <GuessCard
        open={
          held === null
            ? null
            : {
                direction: held.guess.direction,
                startPrice: held.guess.startPrice,
                remainingMs: Math.max(0, held.dueAt - now),
              }
        }
        price={state.price}
        lastResolved={state.lastResolved}
        reducedMotion={reducedMotion}
      />
    </Page>
  );
}
