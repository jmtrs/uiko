# G0 instrumentation

The G0 metric layer is agent-harness neutral.

A harness adapter emits one append-only NDJSON event stream. Each event repeats the same `runId`, `taskId` and `arm`, and sequences are contiguous from 1.

## Why edit events carry text

Final git state is sufficient for `accepted_change_tokens`, but it cannot recover edits that were later reverted or replaced. Therefore every authored edit event carries the actual UTF-8 `beforeText` and `afterText`.

Filesystem notification coalescing is not considered sufficient evidence for `cumulative_authored_edit_tokens`.

## Recording

Pipe one event JSON object at a time:

```bash
echo '{"schemaVersion":1,"sequence":1,...}' | \
  cargo run -p uiko-g0-runner -- append --trace /tmp/G0-D01.ndjson
```

The append command validates the event and enforces contiguous sequence plus stable run/task/arm identity.

## Aggregating

Run from any worktree created from the frozen arm base:

```bash
cargo run -p uiko-g0-runner -- aggregate \
  --repo . \
  --trace /tmp/G0-D01.ndjson \
  --path-policy experiments/g0/path-policy.json \
  --output /tmp/G0-D01.result.json
```

The aggregator independently checks:

- frozen tokenization policy;
- base revision exists;
- edit before/after continuity;
- application final state is represented by the edit trace;
- harness-owned files were not edited;
- final worktree matches each path's final edit;
- public counters and hashes are derived from the trace rather than trusted from the harness.

`filesTouched` is the set of application-owned files touched during the run, including files later reverted.

`changedBytes` and `changedLines` are final application-owned Myers edit distances between the frozen base and final worktree.

`acceptanceLoopActions` is the number of validation plus browser events.

Repair categories are recorded explicitly by the harness/classifier according to `repair-classification.md`; the metric layer does not guess them.
