import { Directions } from "../components/Directions";
import { Masthead } from "../components/Masthead";
import { Page } from "../components/Page";
import { Price } from "../components/Price";
import { Rules } from "../components/Rules";
import { Score } from "../components/Score";
import { canGuess } from "../game/reducer";
import { useGame } from "../game/useGame";

/** The container of the route `/`: wires the game to the presentational components. */
export function GameRoute() {
  const game = useGame();

  return (
    <Page>
      <Masthead>
        <Score score={game.score} />
      </Masthead>
      <Price price={game.price} delayed={game.delayed} />
      <Directions enabled={canGuess(game)} />
      <Rules />
    </Page>
  );
}
