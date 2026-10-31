// Stylelint for the SPA (ADR 0005): a colour is a token, and only the token stylesheet names a raw one.
export default {
  extends: ["stylelint-config-standard", "stylelint-config-css-modules"],
  rules: {
    "color-named": "never",
    "color-no-hex": true,
    "function-disallowed-list": ["rgb", "rgba", "hsl", "hsla", "hwb", "lab", "lch", "oklab", "oklch", "color"],
    "selector-class-pattern": "^[a-z][a-zA-Z0-9]*$",
  },
  overrides: [
    {
      files: ["spa/src/styles/tokens.css"],
      rules: {
        "color-no-hex": null,
      },
    },
  ],
};
