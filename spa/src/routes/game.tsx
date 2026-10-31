import { Masthead } from "../components/Masthead";
import { Page } from "../components/Page";
import { Rules } from "../components/Rules";

/** The container of the route `/`: wires the game to the presentational components. */
export function GameRoute() {
  return (
    <Page>
      <Masthead />
      <Rules />
    </Page>
  );
}
