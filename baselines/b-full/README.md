# B-full baseline

B-full is the strong conventional typed frontend used as the headline comparison for G0.

## Frozen G0 stack

The G0-v1 stack is frozen in `stack-lock.json` and mirrored by exact versions in `package.json`. Semver ranges are intentionally not used.

The selection rule is deliberately conservative toward the conventional arm: prefer a current, maintained and widely adopted stable line over a freshly released major when the newer major could introduce ecosystem churn or reduce coding-agent familiarity.

The frozen stack is:

- Node.js 24.21.0 LTS + npm 11.19.0
- Vite 7.3.6 + @vitejs/plugin-react 5.2.0
- React / React DOM 19.2.8
- TypeScript 5.9.3 in strict mode
- React Router DOM 7.18.4
- TanStack Query 5.103.1
- Zod 4.5.4
- openapi-typescript 7.13.0 + openapi-fetch 0.17.0
- Material UI 7.3.11 + Emotion
- ESLint 10.10.0 + typescript-eslint 8.70.0
- Playwright Test 1.63.0

Before the first measured run, TypeScript was corrected from 6.0.3 to 5.9.3 because the frozen `openapi-typescript` 7.13.0 package declares a TypeScript `^5.x` peer dependency. No measured run used the incompatible combination.

## Frozen executable base

The measured B_FULL base is deliberately strong but task-neutral.

Already wired before a task prompt:

- React/Vite bootstrapping
- BrowserRouter
- TanStack Query provider
- typed `openapi-fetch` client
- generated types from the shared frozen OpenAPI contract
- Material UI
- strict TypeScript and ESLint

The base contains **no customer route or D01-D06 behavior**. Adding routes, queries, projections, filters, pagination and related-order UI remains task-local work.

The API client reads `VITE_API_BASE_URL` and otherwise uses same-origin. A task does not choose or hard-code the harness upstream address.

## Validate the base

```bash
npm install --ignore-scripts --no-audit --no-fund
npm run generate:api
npm run typecheck
npm run lint
npm run build
```

Generated API types and package/build outputs are excluded by the frozen path policy.

## Fairness rules

B-full is not a naive baseline. It may use idiomatic abstractions, generated OpenAPI types, typed fetch helpers, query-key helpers and normal component extraction where a competent 2026 React team would use them.

Every G0 task starts from the same frozen B-full base revision. Tasks are independent rather than cumulative.

Acceptance tests and deterministic network fixtures are harness-owned and are not editable by the coding agent.

After the first measured G0 run starts, dependency or tool changes require a recorded protocol deviation and may not be applied asymmetrically to only one primary arm.
