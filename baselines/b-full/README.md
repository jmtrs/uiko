# B-full baseline

This directory will contain the strong conventional typed frontend used for G0 and final comparison.

Conceptual stack from the v0.16 specification:

- Vite
- React
- TypeScript strict
- TanStack Query
- runtime schema validation (Zod or equivalent)
- OpenAPI-generated client/types
- mature component library
- ESLint + TypeScript compiler
- Playwright-capable acceptance loop

Exact versions are intentionally not frozen in this bootstrap commit. They must be re-verified and pinned before Week-0 experiment freeze, then changed symmetrically only according to the pre-registered protocol.
