import { spawnSync } from "node:child_process";
import { cp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

import { inspectEnvironment, assertEnvironmentMatches, runCapture } from "./environment.mjs";
import {
  cleanupHarnessArtifacts,
  prepareExecutionBase,
} from "./prepare-base.mjs";
import {
  buildCodexArgs,
  runCodexTurn,
  SnapshotObserver,
  TraceWriter,
} from "./trace-adapter.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const defaultRepoRoot = resolve(here, "../../..");

async function main() {
  const { values } = parseArgs({
    options: {
      arm: { type: "string" },
      task: { type: "string" },
      replicate: { type: "string", default: "1" },
      repo: { type: "string", default: defaultRepoRoot },
      results: { type: "string" },
      worktree: { type: "string" },
      codex: { type: "string", default: "codex" },
      lock: {
        type: "string",
        default: resolve(defaultRepoRoot, "experiments/g0/experiment-lock.json"),
      },
    },
  });

  const arm = required(values.arm, "--arm");
  const taskId = required(values.task, "--task");
  const replicate = positiveInteger(values.replicate, "--replicate");
  const sourceRepo = resolve(values.repo);
  const experimentLock = await readJson(resolve(values.lock));
  const adapterLock = await readJson(
    resolve(sourceRepo, "experiments/g0/codex/adapter-lock.json"),
  );
  validateExperimentLock(experimentLock, adapterLock, arm, taskId);

  const runId = `${taskId}-${arm}-r${replicate}`;
  const resultsDir = resolve(
    values.results ?? resolve(tmpdir(), "uiko-g0-results", runId),
  );
  const worktree = resolve(
    values.worktree ?? resolve(tmpdir(), "uiko-g0-worktrees", runId),
  );
  await mkdir(resultsDir, { recursive: true });
  await ensureAbsent(worktree);

  addWorktree(sourceRepo, worktree, experimentLock.harnessRevision);

  let pending;
  try {
    const prerequisiteManifest = await readJson(
      resolve(worktree, "experiments/g0/prerequisite-bases.json"),
    );
    const prepared = prepareExecutionBase({
      repoRoot: worktree,
      arm,
      taskId,
      prerequisiteManifest,
    });
    const expectedBase = experimentLock.taskBases?.[arm]?.[taskId];
    if (expectedBase !== prepared.baseRevision) {
      throw new Error(
        `execution base mismatch for ${arm}/${taskId}: expected ${expectedBase}, got ${prepared.baseRevision}`,
      );
    }

    const environment = await inspectEnvironment(worktree, values.codex);
    assertEnvironmentMatches(environment, experimentLock, adapterLock);

    const runnerPath = resolve(worktree, "target/debug/uiko-g0-runner");
    const tracePath = resolve(resultsDir, "trace.ndjson");
    const writer = new TraceWriter({
      repoRoot: worktree,
      tracePath,
      runnerPath,
      runId,
      taskId,
      arm,
    });

    const task = await frozenTask(worktree, taskId, arm);
    const startedAt = new Date();
    const startedMs = startedAt.getTime();

    await writer.append({
      kind: "run_start",
      baseRevision: prepared.baseRevision,
      startedAt: startedAt.toISOString(),
      startedUnixMs: startedMs,
      model: {
        provider: experimentLock.agent.provider,
        model: experimentLock.agent.model,
        modelVersion: experimentLock.agent.modelVersion,
        agentHarness: `codex-cli/${adapterLock.codexCli.version}+uiko-g0-adapter/v1`,
        parameters: {
          reasoningEffort: experimentLock.agent.reasoningEffort,
          sandbox: adapterLock.execution.sandbox,
          approvalPolicy: adapterLock.execution.approvalPolicy,
          ephemeral: adapterLock.execution.ephemeral,
          agentsEnabled: adapterLock.execution.agentsEnabled,
          unifiedExec: adapterLock.execution.unifiedExec,
        },
        seed: null,
      },
      environment: {
        os: environment.os,
        arch: environment.arch,
        node: environment.node,
        rustc: environment.rustc,
        browser: environment.browser,
        npm: environment.npm,
        codexExecutableSha256: environment.codex.sha256,
      },
    });

    const observer = new SnapshotObserver({
      repoRoot: worktree,
      traceWriter: writer,
    });
    await observer.initialize();

    const codexArgs = buildCodexArgs(
      adapterLock,
      experimentLock.agent.model,
      experimentLock.agent.reasoningEffort,
    );
    await writeFile(
      resolve(resultsDir, "run-config.json"),
      `${JSON.stringify(
        {
          runId,
          arm,
          taskId,
          harnessRevision: experimentLock.harnessRevision,
          prerequisiteRevision: prepared.prerequisiteRevision,
          baseRevision: prepared.baseRevision,
          codexVersion: environment.codex.versionOutput,
          codexExecutable: environment.codex.executable,
          codexExecutableSha256: environment.codex.sha256,
          codexArgs,
          budget: adapterLock.budget,
        },
        null,
        2,
      )}\n`,
      "utf8",
    );

    let prompt = task.prompt;
    let accepted = false;
    let turns = 0;
    const turnResults = [];

    while (
      turns < adapterLock.budget.maxAgentTurns &&
      Date.now() - startedMs < adapterLock.budget.maxWallClockMinutes * 60_000
    ) {
      turns += 1;
      const turnName = `turn-${String(turns).padStart(2, "0")}`;
      await writeFile(resolve(resultsDir, `${turnName}.prompt.txt`), prompt, "utf8");

      const remainingMs =
        adapterLock.budget.maxWallClockMinutes * 60_000 -
        (Date.now() - startedMs);
      const codexResult = await runCodexTurn({
        repoRoot: worktree,
        codexBin: values.codex,
        args: codexArgs,
        prompt,
        rawJsonlPath: resolve(resultsDir, `${turnName}.codex.jsonl`),
        rawStderrPath: resolve(resultsDir, `${turnName}.codex.stderr.log`),
        observer,
        traceWriter: writer,
        timeoutMs: Math.max(1_000, remainingMs),
      });

      if (codexResult.exitCode !== 0) {
        await writer.append({
          kind: "protocol_deviation",
          code: "G0_CODEX_NONZERO_EXIT",
          description: `Codex turn ${turns} exited with code ${codexResult.exitCode}`,
          impact: "material",
        });
      }

      const acceptanceStart = await lastSequence(tracePath);
      const backup = await backupAcceptanceMutableState(
        worktree,
        arm,
        resultsDir,
        turnName,
      );
      const acceptance = await runAcceptance({
        repoRoot: worktree,
        arm,
        taskId,
        tracePath,
        stdoutPath: resolve(resultsDir, `${turnName}.acceptance.stdout.log`),
        stderrPath: resolve(resultsDir, `${turnName}.acceptance.stderr.log`),
      });
      await restoreAcceptanceMutableState(worktree, backup);
      cleanupHarnessArtifacts(worktree, arm);
      await observer.rebaseline();

      const added = await eventsAfter(tracePath, acceptanceStart);
      const failures = acceptanceFailures(added);
      accepted = acceptance.status === 0 && failures.length === 0;

      turnResults.push({
        turn: turns,
        codexExitCode: codexResult.exitCode,
        codexTimedOut: codexResult.timedOut ?? false,
        acceptanceExitCode: acceptance.status,
        acceptanceFailures: failures,
      });

      if (accepted) {
        break;
      }
      if (Date.now() - startedMs >= adapterLock.budget.maxWallClockMinutes * 60_000) {
        break;
      }

      prompt = repairPrompt(failures);
    }

    const candidateOutcome = accepted ? "accepted" : "incomplete";
    const evidence = repairEvidence(await readTrace(tracePath));
    const reviewPath = resolve(resultsDir, "repair-review.pending.json");
    await writeFile(
      reviewPath,
      `${JSON.stringify(
        {
          schemaVersion: 1,
          status: "pending",
          runId,
          taskId,
          arm,
          evidence,
          repairs: [],
          instructions:
            "Review the raw Codex transcript and failed validation/acceptance evidence. Add one primary LOCAL/WIRING/COHERENCE/VISUAL_FIT/ENVIRONMENT category per repaired iteration, set status=reviewed, then run finalize-run.mjs.",
        },
        null,
        2,
      )}\n`,
      "utf8",
    );

    pending = {
      schemaVersion: 1,
      status: "awaiting-repair-review",
      runId,
      taskId,
      arm,
      candidateOutcome,
      turns,
      baseRevision: prepared.baseRevision,
      tracePath,
      worktree,
      resultsDir,
      repairReview: reviewPath,
      turnResults,
    };
    await writeFile(
      resolve(resultsDir, "pending-run.json"),
      `${JSON.stringify(pending, null, 2)}\n`,
      "utf8",
    );
  } catch (error) {
    await writeFile(
      resolve(resultsDir, "harness-error.txt"),
      `${error instanceof Error ? error.stack ?? error.message : String(error)}\n`,
      "utf8",
    );
    throw error;
  }

  process.stdout.write(`${JSON.stringify(pending, null, 2)}\n`);
}

function addWorktree(sourceRepo, worktree, revision) {
  const result = spawnSync(
    "git",
    ["worktree", "add", "--detach", worktree, revision],
    { cwd: sourceRepo, stdio: "inherit" },
  );
  if (result.status !== 0) {
    throw new Error("cannot create isolated G0 worktree");
  }
}

async function frozenTask(repoRoot, taskId, arm) {
  const manifest = await readJson(
    resolve(repoRoot, "experiments/tasks/dev-manifest.json"),
  );
  const task = manifest.tasks.find((candidate) => candidate.id === taskId);
  if (task === undefined) {
    throw new Error(`unknown frozen task ${taskId}`);
  }
  if (
    !manifest.primaryArms.includes(arm) &&
    !task.probes.includes(arm)
  ) {
    throw new Error(`${taskId} is not registered for ${arm}`);
  }
  return task;
}

function validateExperimentLock(lock, adapterLock, arm, taskId) {
  if (lock.schemaVersion !== 1 || lock.status !== "frozen-g0-v1") {
    throw new Error("experiment-lock.json is not frozen-g0-v1");
  }
  if (lock.agent.codexCliVersion !== adapterLock.codexCli.version) {
    throw new Error("experiment lock and Codex adapter lock disagree on CLI version");
  }
  if (
    typeof lock.agent.model !== "string" ||
    lock.agent.model.length === 0 ||
    typeof lock.agent.reasoningEffort !== "string" ||
    lock.agent.reasoningEffort.length === 0
  ) {
    throw new Error("experiment lock must contain an explicit model and reasoning effort");
  }
  if (typeof lock.taskBases?.[arm]?.[taskId] !== "string") {
    throw new Error(`experiment lock has no task base for ${arm}/${taskId}`);
  }
}

async function runAcceptance({
  repoRoot,
  arm,
  taskId,
  tracePath,
  stdoutPath,
  stderrPath,
}) {
  const result = spawnSync(
    process.execPath,
    [
      "experiments/g0/harness/run-acceptance.mjs",
      "--arm",
      arm,
      "--task",
      taskId,
      "--repo",
      repoRoot,
      "--trace",
      tracePath,
    ],
    {
      cwd: repoRoot,
      encoding: "utf8",
      maxBuffer: 16 * 1024 * 1024,
    },
  );
  const stdout = result.stdout ?? "";
  const stderr = result.stderr ?? "";
  await Promise.all([
    writeFile(stdoutPath, stdout, "utf8"),
    writeFile(stderrPath, stderr, "utf8"),
  ]);
  return {
    status: result.status ?? 1,
    stdout,
    stderr,
  };
}

async function backupAcceptanceMutableState(repoRoot, arm, resultsDir, turnName) {
  if (arm !== "T_AUTHORING") {
    return null;
  }
  const source = resolve(repoRoot, "experiments/controls/t-authoring/generated");
  const destination = resolve(resultsDir, `${turnName}.generated-backup`);
  await rm(destination, { recursive: true, force: true });
  await cp(source, destination, { recursive: true });
  return { source, destination };
}

async function restoreAcceptanceMutableState(repoRoot, backup) {
  if (backup === null) {
    return;
  }
  await rm(backup.source, { recursive: true, force: true });
  await cp(backup.destination, backup.source, { recursive: true });
}

function acceptanceFailures(events) {
  const failures = [];
  for (const event of events) {
    if (event.kind === "acceptance" && event.passed === false) {
      failures.push(`criterion ${event.criterionIndex}: ${event.evidence}`);
    }
    if (event.kind === "validation" && event.success === false && event.evidence) {
      failures.push(`acceptance invocation: ${event.evidence}`);
    }
    if (event.kind === "browser" && event.success === false && event.evidence) {
      failures.push(`browser action ${event.action}: ${event.evidence}`);
    }
  }
  return [...new Set(failures)];
}

function repairPrompt(failures) {
  const lines =
    failures.length === 0
      ? ["The frozen acceptance harness did not complete successfully."]
      : failures.map((failure) => `- ${failure}`);
  return [
    "The frozen acceptance harness reported the following deterministic failures after your previous attempt:",
    ...lines,
    "",
    "Continue the same frozen task using the current worktree. Correct the implementation so the acceptance harness passes. Do not modify harness-owned experiment files.",
  ].join("\n");
}

function repairEvidence(events) {
  return events
    .filter(
      (event) =>
        (event.kind === "validation" && event.success === false) ||
        (event.kind === "acceptance" && event.passed === false) ||
        (event.kind === "browser" && event.success === false),
    )
    .map((event) => ({
      sequence: event.sequence,
      kind: event.kind,
      command: event.command ?? null,
      action: event.action ?? null,
      criterionIndex: event.criterionIndex ?? null,
      evidence: event.evidence ?? null,
    }));
}

async function eventsAfter(tracePath, sequence) {
  return (await readTrace(tracePath)).filter((event) => event.sequence > sequence);
}

async function lastSequence(tracePath) {
  const events = await readTrace(tracePath);
  return events.at(-1)?.sequence ?? 0;
}

async function readTrace(tracePath) {
  const text = await readFile(tracePath, "utf8");
  return text
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => JSON.parse(line));
}

async function readJson(path) {
  return JSON.parse(await readFile(path, "utf8"));
}

async function ensureAbsent(path) {
  try {
    await readFile(path);
    throw new Error(`path already exists: ${path}`);
  } catch (error) {
    if (error?.code !== "ENOENT" && error?.code !== "EISDIR") {
      throw error;
    }
  }
  const probe = spawnSync("test", ["!", "-e", path]);
  if (probe.status !== 0) {
    throw new Error(`path already exists: ${path}`);
  }
}

function required(value, option) {
  if (typeof value !== "string" || value.length === 0) {
    throw new Error(`${option} is required`);
  }
  return value;
}

function positiveInteger(value, option) {
  const parsed = Number(value);
  if (!Number.isInteger(parsed) || parsed < 1) {
    throw new Error(`${option} must be a positive integer`);
  }
  return parsed;
}

await main();
