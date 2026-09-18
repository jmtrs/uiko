# G0 Codex execution adapter

This directory contains the coding-agent adapter used for measured G0 runs.

## Frozen harness

The adapter targets Codex CLI **0.155.0** and verifies `codex --version` before a measured run.

The experiment lock supplies the explicit model and reasoning effort. The adapter always uses:

- `codex exec --json`;
- `--ephemeral`;
- `--ignore-user-config`;
- `--ignore-rules`;
- `--sandbox workspace-write`;
- `approval_policy="never"`;
- `agents.enabled=false`;
- `features.unified_exec=false`.

This avoids user configuration, project exec-policy rules, persisted sessions, subagents and unified-exec event hiding.

## Mutation observation

`codex exec --json` is preserved verbatim as JSONL.

The public event stream identifies command and file-change boundaries but does not provide a complete before/after body for every mutation. Therefore the adapter snapshots repository text files:

1. immediately before Codex starts;
2. after every JSONL event boundary;
3. once more when the Codex process exits.

Any changed UTF-8 file becomes one M5 `edit` event with actual `beforeText` and `afterText`.

This is tool-boundary observation, not a filesystem notification watcher. Multiple writes inside one shell command may collapse to the state visible at the next Codex event boundary. Final M5 worktree reconciliation still invalidates unrepresented application changes.

## Acceptance loop

Each run starts with the exact frozen task prompt. After the Codex turn, the shared Playwright acceptance harness runs.

If acceptance fails, the next Codex turn receives only deterministic harness failure evidence. No human implementation hint is injected. The frozen run budget is four Codex turns total.

The trace remains open after the loop. Repair categories are reviewed separately before `run_end` and aggregation.


## Freeze the experiment lock

After this adapter is merged and before the first measured run, freeze the exact
repository revision, task execution bases and machine fingerprint:

```bash
node experiments/g0/codex/freeze-lock.mjs \
  --model gpt-6-astra \
  --reasoning-effort high
```

The defaults are `gpt-6-astra` with `high` reasoning. Codex CLI 0.155.0
supports that pair. The stronger quality-first model is used identically across
paired arms; subagents remain disabled by the adapter lock.

The freezer:

- requires a clean repository and uses its current `HEAD` as
  `harnessRevision`;
- materializes every applicable arm/task execution base through the real setup
  path and records the resulting deterministic commit SHA;
- records the exact Codex executable SHA-256 plus OS, architecture, Node, npm,
  Rust and Chromium versions from the machine that will execute G0;
- writes `experiments/g0/experiment-lock.json`;
- refuses to overwrite a tracked experiment lock.

If the provider exposes an immutable model snapshot identifier, pass it with
`--model-version <snapshot>`. Otherwise the lock records
`modelVersion: "server-managed"` and `modelSnapshotAvailable: false`.

Commit the generated lock before starting the first measured run. After that
point, do not regenerate it silently.
