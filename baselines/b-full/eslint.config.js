import tseslint from "typescript-eslint";

export default tseslint.config(
  {
    ignores: [
      "dist/**",
      "coverage/**",
      "playwright-report/**",
      "test-results/**",
      "src/api/generated/**",
    ],
  },
  ...tseslint.configs.recommended,
);
