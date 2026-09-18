# G0 local experiment

G0 is intentionally independent from any one coding agent.

The frozen core is shared by every run:

- task prompts and probe registration in `experiments/tasks/dev-manifest.json`;
- task prerequisite graph and canonical prerequisite sources;
- `experiments/g0/prepare-base.mjs` for deterministic execution bases;
- the shared Playwright acceptance harness;
- M5 trace and result schemas;
- path policy, repair categories and GO/NO-GO policy;
- the Rust `uiko-g0-runner`.

## Local workflow

Measured runs should be frozen and executed on the same local machine.

1. Choose one agent adapter and authenticate that tool locally.
2. Keep the repository clean.
3. Run the adapter's freeze command. The freeze must run the shared prerequisite preflight before producing a lock.
4. Commit only the resulting experiment lock and frozen setup snapshots in a dedicated lock commit.
5. Run every measured arm from that exact clean lock commit.
6. Preserve the adapter transcript, M5 trace, acceptance evidence and final `result.json`.

Normal CI validates code and deterministic harness pieces. It does not run the exhaustive prerequisite preflight or any real coding agent.

## Agent adapters

Agent-specific code lives below `experiments/g0/<adapter>/`.

An adapter is allowed to differ only where the external coding tool differs. It must not redefine tasks, acceptance criteria, prerequisite behavior, M5 semantics, metric definitions or thresholds.

A measured adapter must:

- record the exact tool/CLI identity and model configuration used locally;
- start from the execution base produced by `prepare-base.mjs`;
- emit `run_start` before delivering the frozen task prompt;
- deliver that prompt without human implementation hints;
- preserve the raw machine-readable transcript when the tool provides one;
- translate observable reads, searches, edits, validations and token usage into the shared M5 trace;
- run the shared Playwright acceptance harness;
- preserve deterministic failure evidence between repair turns;
- leave repair classification for review rather than guessing it;
- finish through the shared result schema and aggregation rules.

The repository currently contains a working `codex/` adapter. It is a reference implementation, not a requirement for G0.

If Claude Code, OpenCode or another local tool is selected, add a sibling adapter only for that tool's invocation, environment fingerprinting and event translation. The shared G0 core should remain unchanged.

## Choosing the tool

Do not mix tools or models within a paired comparison. Choose and freeze the agent/tool/model configuration before the first measured run. If the tool changes, create a new experiment lock and treat it as a separate G0 execution set.
