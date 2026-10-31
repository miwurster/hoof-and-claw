// The acceptance suite's four layers (ADR 0011, ARCHITECTURE.md).
/** @type {import('dependency-cruiser').IConfiguration} */
module.exports = {
  forbidden: [
    {
      name: "cases-speak-only-the-dsl",
      comment: "A test case imports only from dsl/, so it reads in glossary terms.",
      severity: "error",
      from: { path: "^(cases|performance)/" },
      to: { pathNot: "^dsl/" },
    },
    {
      name: "dsl-talks-only-to-the-driver-interface",
      comment: "The DSL knows the driver interface in dsl/drivers.ts, never a driver.",
      severity: "error",
      from: { path: "^dsl/" },
      to: { path: "^(drivers|cases|performance)/" },
    },
    {
      name: "no-driver-imports-another",
      severity: "error",
      from: { path: "^drivers/([^/]+)/" },
      to: { path: "^drivers/", pathNot: "^drivers/$1/" },
    },
    {
      name: "only-the-api-driver-imports-the-game-contract",
      severity: "error",
      from: { pathNot: "^drivers/api/" },
      to: { path: "contract/src/", pathNot: "contract/src/test-controls/" },
    },
    {
      name: "only-the-controls-driver-imports-the-test-controls-contract",
      severity: "error",
      from: { pathNot: "^drivers/controls/" },
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
    enhancedResolveOptions: {
      exportsFields: ["exports"],
      conditionNames: ["import", "types", "default"],
      extensions: [".ts", ".js"],
    },
  },
};
