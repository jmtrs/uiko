# G0 experiment protocol v1

Status: **frozen-g0-v1**  
Freeze date: **2026-09-18**

This protocol operationalizes uiko architecture §4.1 and §24.2–§24.8. G0 is a diagnostic GO/NO-GO gate for the thesis that uiko reduces application implementation a coding agent must invent and reduces cross-layer WIRING/COHERENCE repair before substantial managed-runtime work.

## 1. Question

G0 answers two primary questions:

1. Does the agent author materially less application implementation in uiko than in a strong conventional typed frontend?
2. Does whole-plan compilation reduce real WIRING/COHERENCE repair rather than merely moving work into generated glue, renderer code or browser repair?

G0 does **not** establish production runtime security, review quality, final publication-grade effect sizes or managed-runtime adoption.

## 2. Arms

### B_FULL

Strong conventional frontend baseline frozen in `baselines/b-full/stack-lock.json`.

The implementation may use normal 2026 React engineering practices already available in the frozen stack, including generated OpenAPI types, typed fetch helpers, query-key helpers and component extraction.

### C_UIKO

The minimum uiko compiler/renderer arm required for G0. Application-authored source is the uiko project source under the frozen path policy.

Compiler, renderer and core implementation are infrastructure, not task-local application source. If a development task requires changing those internals, the run records `coreModification=true` and cannot support a positive locality claim.

### R_RENDER_ONLY

Mechanism probe only. Direct json-render composition is combined with conventional TypeScript routing, query and application plumbing. It is run only on the pre-registered probe tasks.

### T_AUTHORING

Mechanism probe only. A minimal TypeScript object/spec facade lowers to the same uiko semantic DTO as JSONC. It is not a second production compiler.

The primary comparison remains **B_FULL vs C_UIKO**.

## 3. Frozen development tasks

The six G0 tasks and exact acceptance criteria are frozen in `experiments/tasks/dev-manifest.json`.

Each task:

- starts from a fresh arm-specific base revision;
- uses the same natural-language task statement in every applicable arm;
- uses the same OpenAPI contract in `experiments/fixtures/g0-support-api.openapi.json`;
- uses deterministic, harness-owned network fixtures;
- is evaluated by equivalent acceptance behavior;
- is independent rather than cumulative.

The coding agent may read the task statement and published acceptance criteria. The executable acceptance harness is read-only and may not be modified during a run.

D01, D02 and D05 additionally run both mechanism probes, satisfying the requirement for at least three probe tasks.

## 4. Pre-run lock

Immediately before the first measured run, record one experiment-lock revision containing:

- B_FULL base commit;
- C_UIKO base commit;
- R/T control base commits;
- exact model/provider/model snapshot where available;
- exact agent harness and version;
- model parameters and seed behavior;
- OS/architecture;
- browser version;
- Rust toolchain for C_UIKO;
- Node/npm versions for TypeScript arms.

The same model and agent harness configuration are used for paired primary-arm runs unless the provider makes this impossible. Any asymmetry is a protocol deviation.

After the first measured run begins, changing task wording, acceptance criteria, path/tokenization rules, package versions, compiler semantics or measurement code requires a recorded material deviation. Silent changes are prohibited.

## 5. Run procedure

For each task and arm:

1. Create an isolated working copy at the frozen arm base revision.
2. Start edit tracing before the task prompt is delivered.
3. Deliver the exact frozen task prompt.
4. Allow normal repository reads/searches, file edits, validation commands and browser use.
5. Record every file mutation with before/after content hashes and path classification.
6. Record every validation/compiler/typecheck/build/browser acceptance attempt.
7. Continue until all acceptance criteria pass, the configured run budget is exhausted or the run becomes invalid.
8. Store one result document conforming to `run-result.schema.json`.
9. Preserve raw transcript/tool logs needed to audit the aggregate metrics.

No human hints about implementation are injected after the task starts. Operational intervention needed to recover the harness is recorded as a protocol deviation.

## 6. Durable authored-change metrics

The frozen token/path policy is `experiments/g0/path-policy.json`.

### Tokenizer

Use `unicode-lexical-v1`:

```text
[\p{L}\p{N}_]+|[^\s]
```

with Unicode matching.

This intentionally avoids a TypeScript-specific or JSON-specific lexer.

### Diff

Tokenize before and after content, then run a Myers diff over the token sequences.

Only tokens on the inserted/replacement side count as authored tokens. Pure deletions contribute zero. This same algorithm is used for both durable metrics.

### accepted_change_tokens

For every application-owned file, compare the frozen base revision with the final accepted state and sum inserted/replacement tokens.

Generated artifacts, lockfiles, build output, harness files and uiko compiler/core internals are excluded by path policy and reported separately when edited.

### cumulative_authored_edit_tokens

For every application-owned file mutation in the complete trace, compare content immediately before and after that mutation and sum inserted/replacement tokens.

Edits that are later reverted, replaced or abandoned still count.

This measures authored repair effort without depending on whether a particular editing tool rewrites an entire file or applies a patch.

## 7. Secondary effort telemetry

Record per run:

- files touched;
- changed bytes and lines;
- repository read operations;
- repository search operations;
- edit operations;
- validation/compiler/typecheck/build invocations;
- browser invocations;
- total acceptance-loop actions;
- wall-clock time;
- provider input/output tokens when complete in every primary arm;
- non-application edit tokens;
- whether generated glue was touched;
- whether uiko core/compiler/runtime internals were modified.

Provider token accounting is descriptive, not a substitute for the durable source metrics.

## 8. Repair classification

Use `experiments/g0/repair-classification.md`.

Each repaired iteration receives exactly one primary category:

- LOCAL
- WIRING
- COHERENCE
- VISUAL_FIT
- ENVIRONMENT

The classification is based on the defect itself, not the file or technology touched.

The task, not the iteration, is the inferential unit. Raw repair iterations must never be bootstrapped or counted as independent samples.

## 9. G0 decision rule

G0 is diagnostic rather than publication-grade. The following conditions are frozen for the mini-set.

### H0: less invented application implementation

For paired B_FULL vs C_UIKO task results:

- median `accepted_change_tokens` reduction must be at least **30%**;
- median `cumulative_authored_edit_tokens` reduction must be at least **30%**;
- C_UIKO completion may not be materially worse than B_FULL;
- the apparent reduction must not be explained by generated/non-application work.

The stricter final-study threshold from architecture §24.7 is not claimed from six development tasks.

### H1: less cross-layer repair

Across the six paired tasks:

- total WIRING + COHERENCE repair iterations in C_UIKO must be at least **30% lower** than B_FULL;
- at least two tasks must show fewer WIRING/COHERENCE repairs in C_UIKO;
- the result does not count if equivalent repair is displaced into VISUAL_FIT/browser work.

### H2: locality

- median application-owned files touched must be lower in C_UIKO than B_FULL;
- no accepted positive G0 result may depend on task-local uiko core/compiler modifications.

### H3: no browser displacement

A positive H0/H1 result is rejected if C_UIKO materially increases VISUAL_FIT repair or acceptance-loop actions enough to erase the operational advantage.

### H4: more than JSON rendering

For D01, D02 and D05 compute the B_FULL→C_UIKO reduction and the B_FULL→R_RENDER_ONLY reduction.

If R_RENDER_ONLY captures **80% or more** of C_UIKO's reduction for both:

- accepted change tokens; and
- WIRING + COHERENCE repairs,

then the G0 result is treated as evidence that JSON rendering explains nearly all measured generation benefit. The product thesis must be narrowed before further runtime investment.

### Authoring-format probe

T_AUTHORING is diagnostic. Compare its syntax/patch failures and cumulative authored-edit tokens with C_UIKO on D01, D02 and D05.

If the TypeScript facade materially removes JSONC-specific repair without losing semantic validation, authoring identity remains open. This does not by itself fail the compiler thesis.

## 10. Completion and invalidation

A run is **accepted** only when every frozen acceptance criterion passes.

A run is **incomplete** when the run budget ends without acceptance.

A run is **invalidated** when, for example:

- executable acceptance tests are modified;
- a sealed/forbidden task source is accessed;
- the task or acceptance criteria are changed mid-run;
- an unrecorded human hint changes implementation;
- the wrong arm/base revision is used.

Invalidated runs are published but excluded from G0 aggregates.

## 11. Protocol deviations

Every deviation is recorded in the result with one impact level:

- `none`
- `minor`
- `material`
- `invalidating`

A material deviation requires re-running all paired conditions affected by it or explicitly abandoning comparability.

## 12. Reporting

Publish task-level raw results for G0, including negative results.

For each primary task report at minimum:

- B_FULL and C_UIKO accepted/cumulative authored tokens;
- paired percentage reductions;
- files touched;
- WIRING + COHERENCE repairs;
- VISUAL_FIT repairs;
- acceptance-loop actions;
- completion outcome;
- deviations;
- core/generated work.

Report R/T probes separately from the primary B_FULL vs C_UIKO table.

Do not present six development tasks as final publication-grade inference. Their purpose is to decide whether building the next product rung is justified.
