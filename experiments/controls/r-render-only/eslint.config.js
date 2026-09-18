import tseslint from "typescript-eslint";

export default tseslint.config(
  {
    ignores: ["dist/**", "src/api/generated/**"],
  },
  ...tseslint.configs.recommended,
);
