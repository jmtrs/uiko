# ADR 0002: Authoring front-ends lower to located source DTOs

- Status: Accepted for PoC
- Date: 2026-09-18

## Context

JSONC is the first PoC authoring surface, but the product thesis must not depend on a proprietary syntax.

## Decision

All authoring front-ends lower to the same located source DTO model before semantic resolution or type/coherence checks.

```text
authoring front-end
  -> CST / parser representation
  -> Located Source DTOs + provenance
  -> semantic compiler
```

## Consequences

- JSONC syntax is not canonical semantics.
- The G0 TypeScript-spec probe can target the same DTO boundary.
- Source spans/provenance are first-class compiler inputs.
