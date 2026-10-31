// The SPA's layers (ARCHITECTURE.md): presentational components take props, and the SPA sees only the game's contract.
/** @type {import('dependency-cruiser').IConfiguration} */
module.exports = {
  forbidden: [
    {
      name: "presentational-components-skip-the-data-layer",
      comment:
        "A presentational component receives only props and callbacks; the route's container wires the game hook.",
      severity: "error",
      from: { path: "^src/components/" },
      to: { path: "^src/(game|routes)/" },
    },
    {
      name: "spa-skips-the-test-controls-contract",
      comment: "The SPA imports only the main export of contract/, never the test controls' types.",
      severity: "error",
      from: { path: "^src/" },
      to: { path: "contract/src/test-controls/" },
    },
    {
      name: "no-circular",
      severity: "error",
      from: {},
      to: { circular: true },
    },
    {
      name: "not-to-unresolvable",
      severity: "error",
      from: {},
      to: { couldNotResolve: true },
    },
  ],
  options: {
    doNotFollow: { path: ["node_modules", "contract/src"] },
    tsConfig: { fileName: "tsconfig.json" },
    tsPreCompilationDeps: true,
    combinedDependencies: true,
    exclude: { path: "\\.test\\.tsx?$" },
    enhancedResolveOptions: {
      exportsFields: ["exports"],
      conditionNames: ["import", "types", "default"],
      extensions: [".ts", ".tsx", ".js", ".css", ".svg"],
    },
  },
};
