# ADR 0001: Rust owns uiko semantic authority

- Status: Accepted for PoC
- Date: 2026-09-18

## Context

The compiler must produce deterministic semantics independent of renderer, browser framework, transport adapter, and authoring syntax.

## Decision

Rust 2024 owns source semantics after authoring adaptation, resolution, type/coherence checking, canonical IR, diagnostics, stable identity, and later the managed runtime.

The web host and custom components do not define compiler semantics.

## Consequences

- `AppIr` must not contain Vue/json-render/Axum/Reqwest-specific concepts.
- Pure semantic passes accept explicit immutable inputs and perform no filesystem/network I/O.
- Framework and protocol integrations terminate at adapters.
