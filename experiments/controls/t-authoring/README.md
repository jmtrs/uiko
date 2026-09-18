# T_AUTHORING control

TypeScript-authoring mechanism probe for G0.

The measured surface is the TypeScript application spec in this directory. It describes the same UIKO semantic concepts as the JSONC arm but uses a typed object representation.

The harness:

1. generates only TypeScript declaration types under `generated/`;
2. runs strict TypeScript typechecking;
3. transpiles the authored TypeScript without a custom runtime;
4. mechanically serializes modules/pages into generated UIKO JSONC;
5. invokes the real UIKO project loader, compiler, UiManifest derivation, gateway and Vue host.

The lowerer does not resolve routes, validate query bindings, understand OpenAPI output shapes, or implement UI semantics. Those responsibilities remain in UIKO.

Generated files are excluded from authored-change metrics by the frozen G0 path policy.
