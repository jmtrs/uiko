# claude (pilot driver only)

`pilot.mjs` drove the 2026-09-18 manual paired pilot recorded in
[`../results/pilot-2026-09-18/`](../results/pilot-2026-09-18/README.md).

This is **not** a measured adapter. It produces no M5 trace, enforces no
lock, and runs a single agent turn per invocation:

```bash
node experiments/g0/claude/pilot.mjs --arm B_FULL --task G0-D01
node experiments/g0/claude/pilot.mjs --arm C_UIKO --task G0-D01
```

Requires: clean tree, `claude` 2.1.198 on PATH, GLM plan key at
`~/.chelper/coding_plan_key.txt` (read at runtime, never recorded). See
[ADR 0004](../../../../docs/adr/0004-g0-pilot-go.md) for why the full
adapter was not built.
