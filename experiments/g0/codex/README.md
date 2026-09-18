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


## Freeze and run locally

Use the same local machine for freezing and the measured G0 runs. This keeps Codex authentication simple and makes the recorded environment the environment that actually executes the experiment.

1. Install the frozen CLI version and authenticate once:

```bash
npm install --global @openai/codex@0.155.0
codex --version
codex login status
# If needed:
codex login
```

2. From a clean checkout, freeze the exact local environment:

```bash
node experiments/g0/codex/freeze-experiment.mjs \
  --provider openai \
  --model <EXACT_CODEX_MODEL> \
  --model-version <PROVIDER_MODEL_VERSION_OR_ALIAS> \
  --reasoning-effort <EFFORT>
```

The freezer itself does not call the model. It:

- requires a clean repository;
- runs predecessor acceptance preflight for every non-empty prerequisite edge;
- fingerprints the Codex executable, Node, npm, Rust and Chromium;
- reconstructs every applicable task execution base and records its exact SHA;
- freezes the exact npm lockfiles under `experiments/g0/frozen-setup/`;
- reconstructs every base a second time with `npm ci` from those snapshots and requires identical SHAs;
- records SHA-256 for every frozen setup snapshot;
- writes `experiments/g0/experiment-lock.json`.

3. Commit only `experiment-lock.json` and `frozen-setup/` in one dedicated lock commit.

4. Run measured tasks from that exact clean lock commit. `run-g0.mjs` reuses the local Codex login and rejects dirty, later or mixed lock checkouts.

The exhaustive prerequisite preflight is intentionally not part of normal CI. It is run by the freezer immediately before the lock is created, which avoids paying the full browser/setup cost on every pull request while preserving the experiment validity check.
