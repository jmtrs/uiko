# R_RENDER_ONLY

Mechanism control for G0.

This arm keeps conventional React routing, query state and API plumbing, but renders task-authored flat UI specs through the official `@json-render/react@0.20.0` renderer.

The frozen base contains no customer route or task behavior.

Application-owned measured paths are `src/**` and optional `ui/**/*.json`. Package/config files are harness-owned during measured runs.
