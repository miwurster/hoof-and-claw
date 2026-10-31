import { createRootRoute, createRoute, createRouter, Navigate, Outlet } from "@tanstack/react-router";
import type { RouterHistory } from "@tanstack/react-router";

import { GameRoute } from "./game";

/** A path the app does not know leads to the game, as the server serves the app at any such path. */
function ToTheGame() {
  return <Navigate to="/" replace />;
}

const rootRoute = createRootRoute({ component: Outlet, notFoundComponent: ToTheGame });

const gameRoute = createRoute({ getParentRoute: () => rootRoute, path: "/", component: GameRoute });

const routeTree = rootRoute.addChildren([gameRoute]);

export function createGameRouter(history?: RouterHistory) {
  return history === undefined ? createRouter({ routeTree }) : createRouter({ routeTree, history });
}

declare module "@tanstack/react-router" {
  interface Register {
    router: ReturnType<typeof createGameRouter>;
  }
}
