import { Directions } from "../components/Directions";
import { GuessMessage } from "../components/GuessMessage";
import { Masthead } from "../components/Masthead";
import { OpenGuessCard } from "../components/OpenGuessCard";
import { Page } from "../components/Page";
import { Price } from "../components/Price";
import { Rules } from "../components/Rules";
import { Score } from "../components/Score";
import { useClock } from "../game/useClock";
import { useGame } from "../game/useGame";

/** The container of the route `/`: wires the game to the presentational components. */
export function GameRoute() {
  const { state, canGuess, place } = useGame();
  const now = useClock(state.openGuess !== null);
  const held = state.openGuess;

  return (
    <Page>
      <Masthead>
        <Score score={state.score} />
      </Masthead>
      <Price price={state.price} delayed={state.delayed} />
      <Directions
        enabled={canGuess}
        onGuess={(direction) => {
          void place(direction);
        }}
      />
      <GuessMessage message={state.message} />
      <Rules />
      {held === null ? null : (
        <OpenGuessCard
          direction={held.guess.direction}
          startPrice={held.guess.startPrice}
          price={state.price}
          remainingMs={Math.max(0, held.dueAt - now)}
        />
      )}
    </Page>
  );
}
