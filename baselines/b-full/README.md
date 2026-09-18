# B-full baseline

B-full is the strong conventional typed frontend used as the headline comparison for G0.

## Frozen G0 stack

The G0-v1 stack is frozen in `stack-lock.json` and mirrored by exact versions in `package.json`. Semver ranges are intentionally not used.

The selection rule is deliberately conservative toward the conventional arm: prefer a current, maintained and widely adopted stable line over a freshly released major when the newer major could introduce ecosystem churn or reduce coding-agent familiarity.

The frozen stack is:

- Node.js 24.21.0 LTS + npm 11.19.0
- Vite 7.3.6 + @vitejs/plugin-react 5.2.0
- React / React DOM 19.2.8
- TypeScript 6.0.3 in strict mode
- React Router DOM 7.18.4
- TanStack Query 5.103.1
- Zod 4.5.4
- openapi-typescript 7.13.0 + openapi-fetch 0.17.0
- Material UI 7.3.11 + Emotion
- ESLint 10.10.0 + typescript-eslint 8.70.0
- Playwright Test 1.63.0

## Fairness rules

B-full is not a naive baseline. It may use idiomatic abstractions, generated OpenAPI types, typed fetch helpers, query-key helpers and normal component extraction where a competent 2026 React team would use them.

Every G0 task starts from the same frozen B-full base revision. Tasks are independent rather than cumulative.

Acceptance tests and deterministic network fixtures are harness-owned and are not editable by the coding agent.

After the first measured G0 run starts, dependency or tool changes require a recorded protocol deviation and may not be applied asymmetrically to only one primary arm.
