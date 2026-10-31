import "@fontsource-variable/geist";
import "./styles/tokens.css";
import "./styles/global.css";

import { RouterProvider } from "@tanstack/react-router";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { createGameRouter } from "./routes/router";

const root = document.querySelector("#root");

if (root === null) {
  throw new Error("index.html has no #root element");
}

createRoot(root).render(
  <StrictMode>
    <RouterProvider router={createGameRouter()} />
  </StrictMode>,
);
