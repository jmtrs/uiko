import { spawnSync } from "node:child_process";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { parseArgs } from "node:util";

import {
  assertEnvironmentMatches,
  inspectEnvironment,
} from "./environment.mjs";
import { prepareExecutionBase } from "./prepare-base.mjs";

const { values } = parseArgs({
  options: {
    repo: { type: "string", default: resolve(".") },
    output: { type: "string" },
    codex: { type: "string", default: "codex" },
    provider: { type: "string" },
    model: { type: "string" },
    "model-version": { type: "string" },
    "reasoning-effort": { type: "string" },
  },
});

const sourceRepo = resolve(values.repo);
const output = resolve(
  values.output ?? join(sourceRepo, "experiments/g0/experiment-lock.json"),
);
const provider = required(values.provider, "--provider");
const model = required(values.model, "--model");
const modelVersion = required(values["model-version"], "--model-version");
const reasoningEffort = required(
  values["reasoning-effort"],
  "--reasoning-effort",
);

requireClean(sourceRepo);

const harnessRevision = capture("git", ["rev-parse", "HEAD"], sourceRepo).trim();
const adapterLock = await readJson(
  join(sourceRepo, "experiments/g0/codex/adapter-lock.json"),
);
const taskManifest = await readJson(
  join(sourceRepo, "experiments/tasks/dev-manifest.json"),
);
const prerequisiteManifest = await readJson(
  join(sourceRepo, "experiments/g0/prerequisite-bases.json"),
);

runPrerequisitePreflight(sourceRepo);

const environment = await inspectEnvironment(sourceRepo, values.codex);
assertEnvironmentMatches(
  environment,
  {
    environment: {
      os: environment.os,
      arch: environment.arch,
      node: environment.node,
      npm: environment.npm,
      rustc: environment.rustc,
      browser: environment.browser,
    },
    agent: {
      codexExecutableSha256: environment.codex.sha256,
    },
  },
  adapterLock,
);

const taskBases = {};
const baseInputs = {};
const root = await mkdtemp(join(tmpdir(), "uiko-g0-lock-"));

try {
  for (const arm of applicableArms(taskManifest)) {
    taskBases[arm] = {};
    baseInputs[arm] = {};

    for (const task of applicableTasks(taskManifest, arm)) {
      const worktree = join(root, `${arm}-${task.id}`);
      addWorktree(sourceRepo, worktree, harnessRevision);

      try {
        const prepared = prepareExecutionBase({
          repoRoot: worktree,
          arm,
          taskId: task.id,
          prerequisiteManifest,
        });
        taskBases[arm][task.id] = prepared.baseRevision;
        baseInputs[arm][task.id] = {
          prerequisiteRevision: prepared.prerequisiteRevision,
          stagedSetupPaths: prepared.stagedSetupPaths,
        };
      } finally {
        removeWorktree(sourceRepo, worktree);
      }
    }
  }
} finally {
  await rm(root, { recursive: true, force: true });
}

const lock = {
  schemaVersion: 1,
  status: "frozen-g0-v1",
  frozenAt: new Date().toISOString(),
  harnessRevision,
  agent: {
    provider,
    model,
    modelVersion,
    reasoningEffort,
    codexCliVersion: adapterLock.codexCli.version,
    codexExecutable: environment.codex.executable,
    codexExecutableSha256: environment.codex.sha256,
  },
  environment: {
    os: environment.os,
    arch: environment.arch,
    node: environment.node,
    npm: environment.npm,
    rustc: environment.rustc,
    browser: environment.browser,
  },
  taskBases,
  baseInputs,
  prerequisitePreflight: {
    status: "passed",
    harnessRevision,
  },
};

await mkdir(dirname(output), { recursive: true });
await writeFile(output, `${JSON.stringify(lock, null, 2)}\n`, "utf8");
process.stdout.write(`${output}\n`);

function runPrerequisitePreflight(repo) {
  const result = spawnSync(
    process.execPath,
    [
      "experiments/g0/harness/validate-prerequisites.mjs",
      "--repo",
      repo,
    ],
    {
      cwd: repo,
      stdio: "inherit",
    },
  );
  if (result.status !== 0) {
    throw new Error("G0 prerequisite acceptance preflight failed");
  }
}

function applicableArms(manifest) {
  return [
    ...manifest.primaryArms,
    ...Object.keys(manifest.mechanismProbes),
  ];
}

function applicableTasks(manifest, arm) {
  if (manifest.primaryArms.includes(arm)) {
    return manifest.tasks;
  }
  return manifest.tasks.filter((task) => task.probes.includes(arm));
}

function addWorktree(repo, worktree, revision) {
  const result = spawnSync(
    "git",
    ["worktree", "add", "--detach", worktree, revision],
    { cwd: repo, encoding: "utf8" },
  );
  if (result.status !== 0) {
    throw new Error(
      `cannot create lock worktree: ${result.stderr || result.stdout}`,
    );
  }
}

function removeWorktree(repo, worktree) {
  const result = spawnSync(
    "git",
    ["worktree", "remove", "--force", worktree],
    { cwd: repo, encoding: "utf8" },
  );
  if (result.status !== 0) {
    throw new Error(
      `cannot remove lock worktree: ${result.stderr || result.stdout}`,
    );
  }
}

function requireClean(repo) {
  const status = capture(
    "git",
    ["status", "--porcelain", "--untracked-files=all"],
    repo,
  );
  if (status.trim().length > 0) {
    throw new Error(
      `repository must be clean before freezing G0:\n${status}`,
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

async function readJson(path) {
  return JSON.parse(await readFile(path, "utf8"));
}

function required(value, option) {
  if (typeof value !== "string" || value.length === 0) {
    throw new Error(`${option} is required`);
  }
  return value;
}
