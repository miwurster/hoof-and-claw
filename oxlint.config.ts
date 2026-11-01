// Oxlint for every TypeScript package (ADR 0012): any warning fails the Green gate.
// anti-slop is vendored in tools/oxlint/anti-slop from dmmulroy/anti-slop at 13909c9e07590f5a85d93c62712259fe14a883f7,
// without its Effect-library rules, which nothing here needs,
// and needs oxlint and @oxlint/plugins at exactly the same version.
import { defineConfig } from "oxlint";

export default defineConfig({
  ignorePatterns: [
    ".agents/**",
    ".claude/**",
    "**/node_modules/**",
    "**/dist/**",
    "acceptance/test-results/**",
    "acceptance/playwright-report/**",
    "contract/src/**",
    "target/**",
    "tools/oxlint/anti-slop/**",
  ],
  plugins: ["typescript", "unicorn", "oxc", "import", "promise", "node"],
  categories: {
    correctness: "error",
    suspicious: "error",
    pedantic: "error",
  },
  jsPlugins: [{ name: "anti-slop", specifier: "./tools/oxlint/anti-slop/index.ts" }],
  rules: {
    "typescript/no-explicit-any": "error",
    "typescript/no-non-null-assertion": "error",
    "typescript/ban-ts-comment": "error",
    "typescript/no-floating-promises": "error",
    "typescript/no-misused-promises": "error",
    "typescript/await-thenable": "error",
    // Off: Playwright's fixtures and React's props are not deeply readonly types, so no code could satisfy it.
    "typescript/prefer-readonly-parameter-types": "off",
    "import/no-unassigned-import": [
      "error",
      { allow: ["**/*.css", "@fontsource-variable/*", "@testing-library/jest-dom/vitest"] },
    ],

    "oxc/no-accumulating-spread": "error",
    "anti-slop/no-array-filter-map": "error",
    "anti-slop/no-reduce-accumulator-copy": "error",
    "anti-slop/no-chained-type-assertions": "error",
    "anti-slop/no-conditional-empty-object-spread": "error",
    "anti-slop/no-known-value-widening": "error",
    "anti-slop/no-module-mocking": "error",
    "anti-slop/no-object-parameters": "error",
    "anti-slop/no-reflect-apply": "error",
    "anti-slop/no-reflect-get": "error",
    "anti-slop/no-runtime-typeof": "error",
    "anti-slop/no-shape-in-symbol-names": "error",
    "anti-slop/no-unknown-parameters": "error",
    "anti-slop/no-unknown-returns": "error",
    "anti-slop/no-unknown-type-aliases": "error",
    "anti-slop/no-unsafe-dictionary-type": "error",
    "anti-slop/no-widen-then-assert": "error",
    "anti-slop/require-readable-spacing": "error",
    "anti-slop/require-safety-comment-for-type-assertion": "error",
  },
  overrides: [
    {
      files: ["spa/**"],
      plugins: ["typescript", "unicorn", "oxc", "import", "promise", "react", "react-perf", "jsx-a11y"],
      rules: {
        "react/rules-of-hooks": "error",
        "react/exhaustive-deps": "error",
        "react/react-in-jsx-scope": "off",
      },
    },
    {
      files: ["acceptance/**"],
      jsPlugins: ["eslint-plugin-playwright"],
      rules: {
        "playwright/no-wait-for-timeout": "error",
        "playwright/no-force-option": "error",
        "playwright/prefer-web-first-assertions": "error",
        "playwright/no-focused-test": "error",
        "playwright/no-skipped-test": "error",
        "playwright/no-networkidle": "error",
        "playwright/no-page-pause": "error",
        "playwright/missing-playwright-await": "error",
      },
    },
  ],
});
