# ADR 0003: G0 and G1 gate substantial runtime investment

- Status: Accepted for PoC
- Date: 2026-09-18

## Decision

Do not build the full managed trust runtime before the product thesis passes two gates:

1. G0: reduced agent-authored implementation plus reduced wiring/coherence repair.
2. G1: complete and useful semantic review surface.

## Rationale

A deterministic runtime is an adoption cost, not proof that the compiler/product is valuable.
