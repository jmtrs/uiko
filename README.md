# uiko

uiko is an experimental compiler-first UI system for coding agents.

Its PoC thesis is intentionally narrow: reduce application behavior an agent must invent, compile the remaining behavior into a deterministic application plan, and test whether that produces smaller repair loops and a better review surface before investing in a managed runtime.

## Current status

**Bootstrap / Week 0.** This repository does not yet claim a working JSONC compiler, renderer, runtime, security boundary, or measured product advantage.

The first engineering milestone is **M0**:

```text
uiko.jsonc
  -> located source DTOs
  -> resolve / type-check / lower
  -> deterministic AppIr
  -> UiManifest
  -> minimal renderer
```

Invalid source must fail before browser execution with stable machine-readable diagnostics.

## Repository shape

```text
crates/uiko-core       semantic primitives and canonical model boundaries
crates/uiko-source     authoring-neutral source DTOs
crates/uiko-compiler   pure semantic compilation passes + diagnostics
crates/uiko-cli        thin CLI shell; no independent semantics
fixtures/              acceptance fixtures, starting with support-console
baselines/b-full       strong conventional frontend comparison arm
experiments/           G0/G1 task manifests and instrumentation material
docs/architecture      product + technical architecture specification
docs/adr               architecture decision records
```

## Build

Rust 2024 is the semantic implementation baseline.

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --locked
```

## Product gates

uiko is gated deliberately:

1. **G0:** show materially less agent-authored implementation and less wiring/coherence repair than a strong typed frontend baseline.
2. **G1:** show that semantic plan diffs are complete and useful to human reviewers.
3. Only then build substantial managed-runtime functionality.

See `docs/architecture/uiko_Product_Technical_Architecture_PoC_v0.16.md` for the normative working specification.
