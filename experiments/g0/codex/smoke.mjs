import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import {
  chmod,
  mkdir,
  mkdtemp,
  readFile,
  rm,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { parseArgs } from "node:util";

import { validateReview } from "./finalize-run.mjs";
import {
  buildCodexArgs,
  diffSnapshots,
  runCodexTurn,
  SnapshotObserver,
  TraceWriter,
} from "./trace-adapter.mjs";

const { values } = parseArgs({
  options: {
    repo: { type: "string", default: resolve(".") },
  },
});

const sourceRepo = resolve(values.repo);
const runnerPath = resolve(sourceRepo, "target/debug/uiko-g0-runner");
const root = await mkdtemp(join(tmpdir(), "uiko-g0-codex-smoke-"));
const worktree = join(root, "repo");
const results = join(root, "results");

try {
  await mkdir(worktree, { recursive: true });
  await mkdir(results, { recursive: true });

  runChecked("git", ["init", "--quiet"], worktree);
  runChecked("git", ["config", "user.name", "uiko smoke"], worktree);
  runChecked("git", ["config", "user.email", "smoke@uiko.invalid"], worktree);
  await writeFile(join(worktree, "app.txt"), "before\n", "utf8");
  runChecked("git", ["add", "app.txt"], worktree);
  runChecked("git", ["commit", "--quiet", "-m", "base"], worktree);
  const baseRevision = capture("git", ["rev-parse", "HEAD"], worktree).trim();

  const fakeCodex = join(root, "fake-codex.mjs");
  await writeFile(
    fakeCodex,
    `#!/usr/bin/env node
import { writeFileSync } from "node:fs";

const args = process.argv.slice(2);
if (args.includes("--version")) {
  process.stdout.write("codex-cli 0.155.0\\n");
  process.exit(0);
}
if (args[0] !== "exec" || !args.includes("--json")) {
  process.stderr.write("unexpected fake Codex invocation\\n");
  process.exit(2);
}

const emit = (value) => process.stdout.write(JSON.stringify(value) + "\\n");
writeFileSync("app.txt", "after\\n", "utf8");
emit({ type: "thread.started", thread_id: "smoke-thread" });
emit({
  type: "item.completed",
  item: {
    id: "read-1",
    type: "command_execution",
    command: "cat app.txt",
    aggregated_output: "after\\n",
    exit_code: 0,
    status: "completed"
  }
});
emit({
  type: "item.completed",
  item: {
    id: "search-1",
    type: "command_execution",
    command: "rg needle .",
    aggregated_output: "",
    exit_code: 0,
    status: "completed"
  }
});
emit({
  type: "item.completed",
  item: {
    id: "validation-1",
    type: "command_execution",
    command: "npm run typecheck",
    aggregated_output: "synthetic typecheck failure",
    exit_code: 1,
    status: "failed"
  }
});
emit({
  type: "turn.completed",
  usage: { input_tokens: 11, output_tokens: 7 }
});
process.stdout.write("not-json\\n");
`,
    "utf8",
  );
  await chmod(fakeCodex, 0o755);

  const tracePath = join(results, "trace.ndjson");
  const writer = new TraceWriter({
    repoRoot: worktree,
    tracePath,
    runnerPath,
    runId: "G0-D01-B_FULL-r1-smoke",
    taskId: "G0-D01",
    arm: "B_FULL",
  });
  await writer.append({
    kind: "run_start",
    baseRevision,
    startedAt: "2000-01-01T00:00:00.000Z",
    startedUnixMs: 946684800000,
    model: {
      provider: "smoke",
      model: "smoke",
      modelVersion: "smoke",
      agentHarness: "codex-cli/0.155.0+uiko-g0-adapter/v1",
      parameters: {},
      seed: null,
    },
    environment: {
      os: process.platform,
      arch: process.arch,
      node: process.version,
      rustc: null,
      browser: null,
    },
  });

  const observer = new SnapshotObserver({ repoRoot: worktree, traceWriter: writer });
  await observer.initialize();

  const lock = {
    execution: {
      sandbox: "workspace-write",
      approvalPolicy: "never",
      agentsEnabled: false,
      unifiedExec: false,
    },
  };
  const args = buildCodexArgs(lock, "smoke-model", "high");
  for (const required of [
    "exec",
    "--json",
    "--ephemeral",
    "--ignore-user-config",
    "--ignore-rules",
    "--sandbox",
    "workspace-write",
    "--model",
    "smoke-model",
  ]) {
    assert.ok(args.includes(required), `missing frozen Codex argument: ${required}`);
  }
  assert.ok(args.includes('approval_policy="never"'));
  assert.ok(args.includes('model_reasoning_effort="high"'));
  assert.ok(args.includes("agents.enabled=false"));
  assert.ok(args.includes("features.unified_exec=false"));

  const rawJsonlPath = join(results, "codex.jsonl");
  const rawStderrPath = join(results, "codex.stderr.log");
  const outcome = await runCodexTurn({
    repoRoot: worktree,
    codexBin: fakeCodex,
    args,
    prompt: "smoke prompt",
    rawJsonlPath,
    rawStderrPath,
    observer,
    traceWriter: writer,
    timeoutMs: 5_000,
  });

  assert.equal(outcome.exitCode, 0);
  assert.equal(outcome.timedOut, false);
  assert.equal(outcome.parseErrors, 1);
  assert.equal(outcome.threadId, "smoke-thread");

  const events = (await readFile(tracePath, "utf8"))
    .trim()
    .split(/\r?\n/)
    .map((line) => JSON.parse(line));

  assert.ok(
    events.some(
      (event) =>
        event.kind === "edit" &&
        event.path === "app.txt" &&
        event.operation === "update" &&
        event.beforeText === "before\n" &&
        event.afterText === "after\n",
    ),
  );
  assert.ok(
    events.some(
      (event) => event.kind === "repository_read" && event.target === "cat app.txt",
    ),
  );
  assert.ok(
    events.some(
      (event) => event.kind === "repository_search" && event.query === "rg needle .",
    ),
  );
  assert.ok(
    events.some(
      (event) =>
        event.kind === "validation" &&
        event.command === "npm run typecheck" &&
        event.success === false &&
        event.evidence === "synthetic typecheck failure",
    ),
  );
  assert.ok(
    events.some(
      (event) =>
        event.kind === "provider_tokens" &&
        event.inputTokens === 11 &&
        event.outputTokens === 7,
    ),
  );
  assert.ok(
    events.some(
      (event) =>
        event.kind === "protocol_deviation" &&
        event.code === "G0_CODEX_JSON_PARSE" &&
        event.impact === "material",
    ),
  );

  const raw = await readFile(rawJsonlPath, "utf8");
  assert.match(raw, /"thread.started"/);
  assert.match(raw, /not-json/);

  assert.deepEqual(
    diffSnapshots(
      new Map([
        ["a.txt", "one"],
        ["gone.txt", "old"],
      ]),
      new Map([
        ["a.txt", "two"],
        ["new.txt", "new"],
      ]),
    ).map(({ path, operation }) => ({ path, operation })),
    [
      { path: "a.txt", operation: "update" },
      { path: "gone.txt", operation: "delete" },
      { path: "new.txt", operation: "create" },
    ],
  );

  const pending = {
    status: "awaiting-repair-review",
    runId: "G0-D01-B_FULL-r1-smoke",
    turns: 2,
  };
  assert.doesNotThrow(() =>
    validateReview(pending, {
      schemaVersion: 1,
      status: "reviewed",
      runId: pending.runId,
      reviewer: "smoke",
      repairs: [
        {
          iteration: 1,
          category: "WIRING",
          reason: "synthetic",
          evidence: "synthetic",
          sourcePaths: ["app.txt"],
        },
      ],
    }),
  );
  assert.throws(
    () =>
      validateReview(pending, {
        schemaVersion: 1,
        status: "reviewed",
        runId: pending.runId,
        reviewer: "smoke",
        repairs: [
          {
            iteration: 2,
            category: "WIRING",
            reason: "synthetic",
            evidence: "synthetic",
            sourcePaths: ["app.txt"],
          },
        ],
      }),
    /exceeds the 1 possible repair iteration/,
  );

  process.stdout.write("Codex G0 adapter smoke passed\n");
} finally {
  await rm(root, { recursive: true, force: true });
}

function runChecked(command, args, cwd) {
  const result = spawnSync(command, args, { cwd, encoding: "utf8" });
  if (result.status !== 0) {
    throw new Error(
      `${command} ${args.join(" ")} failed: ${result.stderr || result.stdout}`,
    );
  }
}

function capture(command, args, cwd) {
  const result = spawnSync(command, args, { cwd, encoding: "utf8" });
  if (result.status !== 0) {
    throw new Error(
      `${command} ${args.join(" ")} failed: ${result.stderr || result.stdout}`,
    );
  }
  return result.stdout;
}
