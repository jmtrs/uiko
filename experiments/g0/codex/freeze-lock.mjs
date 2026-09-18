import { spawnSync } from "node:child_process";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

import { inspectEnvironment, runCapture } from "./environment.mjs";
import { prepareExecutionBase } from "./prepare-base.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const defaultRepoRoot = resolve(here, "../../..");

const { values } = parseArgs({
  options: {
    repo: { type: "string", default: defaultRepoRoot },
    output: { type: "string" },
    codex: { type: "string", default: "codex" },
    model: { type: "string", default: "gpt-6-astra" },
    "model-version": { type: "string" },
    "reasoning-effort": { type: "string", default: "high" },
  },
});

const sourceRepo = resolve(values.repo);
const outputPath = resolve(
  values.output ?? resolve(sourceRepo, "experiments/g0/experiment-lock.json"),
);
const codexBin = values.codex;
const model = nonEmpty(values.model, "--model");
const reasoningEffort = nonEmpty(
  values["reasoning-effort"],
  "--reasoning-effort",
);
const explicitModelVersion = values["model-version"];
const modelVersion =
  explicitModelVersion === undefined
    ? "server-managed"
    : nonEmpty(explicitModelVersion, "--model-version");

requireClean(sourceRepo);
requireUntrackedOutput(sourceRepo, outputPath);

const harnessRevision = runCapture(
  "git",
  ["rev-parse", "HEAD"],
  sourceRepo,
).trim();
const adapterLock = await readJson(
  resolve(sourceRepo, "experiments/g0/codex/adapter-lock.json"),
);
const taskManifest = await readJson(
  resolve(sourceRepo, "experiments/tasks/dev-manifest.json"),
);
const prerequisiteManifest = await readJson(
  resolve(sourceRepo, "experiments/g0/prerequisite-bases.json"),
);

assertFrozenInputs(adapterLock, taskManifest, prerequisiteManifest);

const scratchRoot = await mkdtemp(
  resolve(tmpdir(), "uiko-g0-experiment-lock-"),
);
const worktree = resolve(scratchRoot, "worktree");
let environment;
const taskBases = {};
const predecessorAcceptance = {};

try {
  addWorktree(sourceRepo, worktree, harnessRevision);

  for (const arm of frozenArms(taskManifest)) {
    taskBases[arm] = {};

    for (const task of tasksForArm(taskManifest, arm)) {
      resetWorktree(worktree, harnessRevision);
      const prepared = prepareExecutionBase({
        repoRoot: worktree,
        arm,
        taskId: task.id,
        prerequisiteManifest,
      });
      taskBases[arm][task.id] = prepared.baseRevision;

      const prerequisiteStage = prerequisiteManifest.taskBases[arm][task.id];
      if (prerequisiteStage !== "EMPTY") {
        const predecessorTask = predecessorTaskForStage(prerequisiteStage);
        runPrerequisiteAcceptance(worktree, arm, predecessorTask);
        predecessorAcceptance[arm] ??= {};
        predecessorAcceptance[arm][task.id] = {
          baseRevision: prepared.baseRevision,
          predecessorTask,
          passed: true,
        };
      }
    }
  }

  environment = await inspectEnvironment(worktree, codexBin);
  if (!environment.codex.versionOutput.includes(adapterLock.codexCli.version)) {
    throw new Error(
      `Codex version mismatch: expected ${adapterLock.codexCli.version}, got ${environment.codex.versionOutput}`,
    );
  }
} finally {
  removeWorktree(sourceRepo, worktree);
  await rm(scratchRoot, { recursive: true, force: true });
}

const lock = {
  schemaVersion: 1,
  status: "frozen-g0-v1",
  frozenAt: new Date().toISOString(),
  harnessRevision,
  taskBases,
  predecessorAcceptance,
  agent: {
    provider: "openai",
    codexCliVersion: adapterLock.codexCli.version,
    codexExecutableSha256: environment.codex.sha256,
    model,
    modelVersion,
    modelSnapshotAvailable: explicitModelVersion !== undefined,
    reasoningEffort,
  },
  environment: {
    os: environment.os,
    arch: environment.arch,
    node: environment.node,
    npm: environment.npm,
    rustc: environment.rustc,
    browser: environment.browser,
  },
};

await writeFile(outputPath, `${JSON.stringify(lock, null, 2)}\n`, "utf8");
process.stdout.write(
  `Frozen G0 experiment lock written to ${outputPath}\n` +
    `harnessRevision=${harnessRevision}\n` +
    `codex=${environment.codex.versionOutput}\n` +
    `codexSha256=${environment.codex.sha256}\n`,
);

function predecessorTaskForStage(stage) {
  const mapping = {
    D01: "G0-D01",
    D02: "G0-D02",
    D03: "G0-D03",
  };
  const taskId = mapping[stage];
  if (taskId === undefined) {
    throw new Error(`no predecessor acceptance mapping for stage ${stage}`);
  }
  return taskId;
}

function runPrerequisiteAcceptance(repoRoot, arm, taskId) {
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
      "--prerequisite",
    ],
    {
      cwd: repoRoot,
      encoding: "utf8",
      maxBuffer: 16 * 1024 * 1024,
    },
  );
  if (result.status !== 0) {
    throw new Error(
      `predecessor acceptance failed for ${arm}/${taskId}:\n${result.stdout || ""}\n${result.stderr || ""}`,
    );
  }
}

function frozenArms(manifest) {
  return [
    ...manifest.primaryArms,
    ...Object.keys(manifest.mechanismProbes),
  ];
}

function tasksForArm(manifest, arm) {
  if (manifest.primaryArms.includes(arm)) {
    return manifest.tasks;
  }
  return manifest.tasks.filter((task) => task.probes.includes(arm));
}

function assertFrozenInputs(adapterLockValue, taskManifestValue, prerequisiteValue) {
  if (
    adapterLockValue.schemaVersion !== 1 ||
    adapterLockValue.status !== "frozen-g0-v1"
  ) {
    throw new Error("Codex adapter lock is not frozen-g0-v1");
  }
  if (
    taskManifestValue.schemaVersion !== 2 ||
    taskManifestValue.status !== "frozen-g0-v1"
  ) {
    throw new Error("G0 task manifest is not frozen-g0-v1 schemaVersion=2");
  }
  if (
    prerequisiteValue.schemaVersion !== 1 ||
    prerequisiteValue.status !== "pre-measurement-g0-v1"
  ) {
    throw new Error("G0 prerequisite manifest is not the expected pre-measurement version");
  }

  const expectedArms = new Set(frozenArms(taskManifestValue));
  for (const arm of expectedArms) {
    if (prerequisiteValue.taskBases?.[arm] === undefined) {
      throw new Error(`prerequisite manifest is missing arm ${arm}`);
    }
    for (const task of tasksForArm(taskManifestValue, arm)) {
      if (typeof prerequisiteValue.taskBases[arm][task.id] !== "string") {
        throw new Error(
          `prerequisite manifest is missing ${arm}/${task.id}`,
        );
      }
    }
  }
}

function addWorktree(repoRoot, target, revision) {
  runChecked(
    "git",
    ["worktree", "add", "--detach", target, revision],
    repoRoot,
  );
}

function resetWorktree(repoRoot, revision) {
  runChecked("git", ["reset", "--hard", "--quiet", revision], repoRoot);
  const status = runCapture(
    "git",
    ["status", "--porcelain", "--untracked-files=all"],
    repoRoot,
  );
  if (status.trim().length > 0) {
    throw new Error(
      `temporary G0 worktree is not clean after reset:\n${status}`,
    );
  }
}

function removeWorktree(repoRoot, target) {
  const result = spawnSync(
    "git",
    ["worktree", "remove", "--force", target],
    { cwd: repoRoot, encoding: "utf8" },
  );
  if (result.status !== 0 && !/is not a working tree/i.test(result.stderr ?? "")) {
    throw new Error(
      `cannot remove temporary G0 worktree: ${result.stderr || result.stdout}`,
    );
  }
}

function requireClean(repoRoot) {
  const status = runCapture(
    "git",
    ["status", "--porcelain", "--untracked-files=all"],
    repoRoot,
  );
  if (status.trim().length > 0) {
    throw new Error(
      `experiment lock must be frozen from a clean repository:\n${status}`,
    );
  }
}

function requireUntrackedOutput(repoRoot, path) {
  const relativePath = relative(repoRoot, path).replaceAll("\\", "/");
  if (
    relativePath.startsWith("../") ||
    relativePath === ".." ||
    relativePath.length === 0
  ) {
    throw new Error("--output must be a new file inside the repository");
  }

  const result = spawnSync(
    "git",
    ["ls-files", "--error-unmatch", "--", relativePath],
    { cwd: repoRoot, stdio: "ignore" },
  );
  if (result.status === 0) {
    throw new Error(
      `${relativePath} is already tracked; refuse to silently move the frozen harness revision`,
    );
  }
  if (result.status !== 1) {
    throw new Error(`cannot determine whether ${relativePath} is tracked`);
  }
}

function runChecked(command, args, cwd) {
  const result = spawnSync(command, args, { cwd, stdio: "inherit" });
  if (result.status !== 0) {
    throw new Error(
      `${command} ${args.join(" ")} failed with exit code ${result.status}`,
    );
  }
}

async function readJson(path) {
  return JSON.parse(await readFile(path, "utf8"));
}

function nonEmpty(value, option) {
  if (typeof value !== "string" || value.length === 0) {
    throw new Error(`${option} must be a non-empty string`);
  }
  return value;
}
