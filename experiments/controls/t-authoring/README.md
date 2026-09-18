# T_AUTHORING

Mechanism control for G0.

The measured author edits only `src/**/*.ts`. The TypeScript object model mirrors the supported uiko source model. The harness lowerer performs deterministic file materialization only; semantic validation and compilation are delegated to the normal uiko project loader/compiler.

Generated JS, generated uiko source, manifests, package metadata and the lowering harness are not authored application surface.
