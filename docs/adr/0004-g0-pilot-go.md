# ADR 0004: G0 pilot GO without a measured adapter

- Status: Accepted
- Date: 2026-09-18

## Context

Issue #37 asked to pick a local agent/model configuration, build a measured
G0 adapter, freeze an experiment lock and run the six paired tasks. A full
`experiments/g0/claude/` adapter had been designed (mirroring the `codex/`
reference adapter) when a cheaper probe became obvious: before building the
instrument, run one manual paired execution of G0-D01 (B_FULL vs C_UIKO)
with the same frozen core and see whether the dominant risk is even live.

The dominant risk: the selected model (`glm-5.2[1m]`, chosen because tokens
were exhausted elsewhere) has no uiko in training. If it cannot learn the
jsonc authoring format from repository context, every measured number that
follows is noise.

## Decision

1. **G0 is a GO on pilot evidence.** The owner accepts the paired pilot
   (2026-09-18, both arms accepted 4/4, ~3× application-token reduction in
   C_UIKO, consistent with the canonical ceiling) as sufficient decision
   signal for the PoC's next step. Evidence and limitations:
   `experiments/g0/results/pilot-2026-09-18/`.
2. **The claude adapter will not be built now.** The pilot driver
   (`experiments/g0/claude/pilot.mjs`) stays as the reproduction tool.
3. **The measured G0 path stays frozen and available.** If
   publication-grade numbers or the H0–H4 protocol evaluation
   (`experiments/g0/protocol-v1.md` §9) are ever needed — including the
   R_RENDER_ONLY "JSON rendering explains it" probe — the shared core,
   `codex/` reference adapter and the designed claude adapter plan remain
   the route. Nothing in the core was modified.

## Consequences

- Proceed to the next architecture bet on the strength of the pilot;
  the G0 gate is considered passed at pilot grade, not protocol grade.
- Any external claim of "measured G0 results" would be false: the pilot is
  n=1, one task, no M5 trace, no probes. The pilot README states this.
- The pilot surfaced one open protocol question — lockfile churn from
  `cargo build` classified as `other`/`excluded` — to be decided if the
  measured path is ever revived.
- The cost-structure observation (C_UIKO reads 3× more input to write 3×
  less application code) is expected by the thesis's reads-vs-edits
  decomposition, not evidence against it; provider tokens remain
  descriptive only.
