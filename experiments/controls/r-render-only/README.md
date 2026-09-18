# R_RENDER_ONLY control

Renderer-only mechanism probe for G0.

This arm intentionally keeps the conventional frontend responsibilities from B_FULL:

- React Router owns routing and route parameters.
- TanStack Query owns async query lifecycle/cache/refetch.
- `openapi-fetch` uses generated OpenAPI types.
- React/TypeScript owns application state and orchestration.

The only mechanism changed is UI tree rendering: task code may compose flat json-render specs and render them through the frozen generic `@json-render/react@0.20.0` registry.

The frozen base contains no customer route, operation, filter, pagination or task-specific action.

Measured application paths are defined by `experiments/g0/path-policy.json`. Generated OpenAPI types and package-lock output are excluded from authored-change metrics.
