# G0 prerequisite bases

These files are harness-owned canonical predecessor implementations for frozen G0 tasks that explicitly extend earlier behavior.

They are **not** measured application paths. Before a successor run, `materialize-prerequisite-base.sh` copies the selected template into the normal measured arm root and creates a deterministic synthetic git commit. That synthetic commit becomes the run's `baseRevision`.

## Graph

- D01: empty arm base
- D02: empty arm base
- D03: canonical D01
- D04: canonical D02
- D05: canonical D03
- D06: canonical D02

The same graph applies to mechanism probes where those tasks exist. Control-arm D03 templates are added with their control implementations.

## Why

D03, D04, D05 and D06 are worded as extensions of already-existing UI. Measuring them from an empty project would count unstated prerequisite work and make task-level comparisons misleading.

## Determinism

Synthetic commits use:

- the experiment-lock checkout as parent
- the materialized measured-arm tree
- fixed author/committer identity
- fixed timestamp
- fixed commit-message format

Therefore the same experiment lock, arm and task produce the same prerequisite base SHA.

The measured agent may edit the materialized application files. It may not edit these harness-owned templates.
