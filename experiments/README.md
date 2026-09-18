# Experiments

This directory is part of the product, not benchmark decoration.

> G0 status: **GO at pilot grade** (2026-09-18, ADR 0004). Evidence:
> `g0/results/pilot-2026-09-18/`. The frozen definitions below were not
> modified; no experiment lock was created.

## G0 freeze

The first G0 experiment definition is frozen as `g0-v1`:

- `g0/protocol-v1.md` — run procedure and GO/NO-GO rules;
- `g0/path-policy.json` — authored-source, exclusion and tokenization rules;
- `g0/repair-classification.md` — LOCAL / WIRING / COHERENCE / VISUAL_FIT / ENVIRONMENT;
- `g0/run-result.schema.json` — arm-neutral result format;
- `g0/trace-event.schema.json` — arm-neutral append-only event format;
- `g0/instrumentation.md` — trace/aggregation contract;
- `g0/run-template.json` — result skeleton;
- `tasks/dev-manifest.json` — six pre-registered G0 development tasks;
- `fixtures/g0-support-api.openapi.json` — shared read-only API contract;
- `fixtures/compiler-support-console/` — harness-owned regression fixture for compiler/renderer/runtime CI; never a measured arm base;
- `../baselines/b-full/stack-lock.json` — exact conventional baseline versions.

Primary durable task-level measures for G0/final evaluation:

- `accepted_change_tokens`
- `cumulative_authored_edit_tokens`
- files touched
- repository read/search work
- WIRING + COHERENCE repair iterations
- browser / VISUAL_FIT iterations
- completion outcome

Every G0 task starts from a fresh frozen arm base; tasks are not cumulative.

R-render-only and T-authoring are mechanism probes on D01, D02 and D05. They are not headline experimental arms.

Hold-out tasks remain sealed until their measurement window. G0 development tasks are intentionally readable because they are used to tune and decide whether the product thesis justifies further investment.
