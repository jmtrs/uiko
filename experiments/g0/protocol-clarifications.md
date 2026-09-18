# G0 pre-measurement protocol clarifications

This file records corrections discovered before the first measured G0 run. It does not rewrite task prompts, acceptance criteria, metric definitions or GO/NO-GO thresholds.

## 2026-09-18: task-specific prerequisite bases

The original protocol states that tasks are independent and begin from a fresh frozen arm base. The frozen development prompts also make D03, D04, D05 and D06 explicit extensions of prior UI behavior.

The operational interpretation is therefore:

- a task is independent from previous **measured runs**;
- each run starts from a fresh checkout of its frozen **task-specific prerequisite base**;
- prerequisite behavior is canonical harness-owned setup, not copied from a previous agent's output;
- the trace begins only after that base is materialized;
- `baseRevision` is the task-specific synthetic base commit;
- accepted/cumulative edit metrics therefore measure only work performed after the successor prompt is delivered.

Frozen graph:

| Task | Prerequisite |
| --- | --- |
| D01 | empty arm base |
| D02 | empty arm base |
| D03 | canonical D01 |
| D04 | canonical D02 |
| D05 | canonical D03 |
| D06 | canonical D02 |

The mechanism probes use the same dependency semantics for D01, D02 and D05.

No measured G0 run preceded this clarification.
