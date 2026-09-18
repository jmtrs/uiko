# G0 acceptance harness

This directory is harness-owned and read-only during measured runs.

## Run

Preconditions:

- the measured arm worktree is already at the task-specific frozen base and contains the agent's candidate change;
- normal arm dependencies are installed before tracing starts;
- Chromium for `@playwright/test@1.63.0` is installed;
- when `--trace` is used, the trace already contains `run_start` and has not yet emitted `run_end`.

From the repository root:

```bash
cd experiments/g0/harness
npm install --ignore-scripts --no-audit --no-fund
npx playwright install chromium

node run-acceptance.mjs --arm B_FULL --task G0-D01 --repo ../../..
node run-acceptance.mjs --arm C_UIKO --task G0-D01 --repo ../../..
```

For a measured run:

```bash
node run-acceptance.mjs \
  --arm C_UIKO \
  --task G0-D05 \
  --repo ../../.. \
  --trace /tmp/G0-D05-C_UIKO-r1.ndjson
```

## Shared semantics

Both primary arms use:

- the same deterministic fixture API on port 4317;
- the same customer/order records;
- the same request journal;
- the same task-indexed acceptance functions;
- the same browser engine;
- the same criteria indices as `experiments/tasks/dev-manifest.json`.

Only launch plumbing differs:

- B_FULL receives the fixture base URL through `VITE_API_BASE_URL`;
- C_UIKO compiles its UiManifest, starts the uiko G0 gateway against the same fixture, and serves the Vue host.

Acceptance intentionally avoids framework-private selectors. It uses roles, labels, visible data and fixture request evidence. D01/D02 additionally inspect measured source paths to reject hard-coded fixture/customer IDs.

## Trace events

When `--trace` is present, the harness appends:

- one `browser` event for each explicit browser interaction;
- one `acceptance` event per criterion reached;
- one final `validation` event for the acceptance invocation.

A task passes only if every frozen criterion for that task was reached and passed.
