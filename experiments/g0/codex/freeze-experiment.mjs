import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { parseArgs } from "node:util";

import { inspectEnvironment } from "./environment.mjs";
import { prepareExecutionBase } from "./prepare-base.mjs";

const REQUIRED_NPM_LOCKS = [
  "experiments/g0/harness/package-lock.json",
  "baselines/b-full/package-lock.json",
  "hosts/vue/package-lock.json",
  "experiments/controls/r-render-only/package-lock.json",
];

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
const harnessPackage = await readJson(
  join(sourceRepo, "experiments/g0/harness/package.json"),
);

runPrerequisitePreflight(sourceRepo);

let environment = null;
const taskBases = {};
const baseInputs = {};
const npmLocks = new Map();
const generationRoot = await mkdtemp(join(tmpdir(), "uiko-g0-lock-generation-"));

try {
  for (const arm of applicableArms(taskManifest)) {
    taskBases[arm] = {};
    baseInputs[arm] = {};

    for (const task of applicableTasks(taskManifest, arm)) {
      const worktree = join(generationRoot, `${arm}-${task.id}`);
      addWorktree(sourceRepo, worktree, harnessRevision);

      try {
        const prepared = prepareExecutionBase({
          repoRoot: worktree,
          arm,
          taskId: task.id,
          prerequisiteManifest,
        });

        if (environment === null) {
          environment = await inspectEnvironment(worktree, values.codex);
          assertFrozenEnvironment(environment, adapterLock, harnessPackage);
        }

        await captureNpmLocks(worktree, prepared.stagedSetupPaths, npmLocks);
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
  await rm(generationRoot, { recursive: true, force: true });
}

if (environment === null) {
  throw new Error("no applicable G0 task bases were produced");
}
for (const path of REQUIRED_NPM_LOCKS) {
  if (!npmLocks.has(path)) {
    throw new Error(`freezer did not capture required npm lock ${path}`);
  }
}

const verificationSnapshots = await mkdtemp(
  join(tmpdir(), "uiko-g0-frozen-setup-"),
);
try {
  await writeSetupSnapshots(verificationSnapshots, npmLocks);
  await verifyFrozenTaskBases({
    sourceRepo,
    harnessRevision,
    prerequisiteManifest,
    taskManifest,
    expectedTaskBases: taskBases,
    frozenSetupRoot: verificationSnapshots,
  });
} finally {
  await rm(verificationSnapshots, { recursive: true, force: true });
}

const frozenSetupRoot = join(sourceRepo, "experiments/g0/frozen-setup");
await rm(frozenSetupRoot, { recursive: true, force: true });
await writeSetupSnapshots(frozenSetupRoot, npmLocks);

const setupSnapshots = Object.fromEntries(
  [...npmLocks.entries()]
    .sort(([left], [right]) => left.localeCompare(right))
    .map(([path, bytes]) => [
      path,
      {
        path: `experiments/g0/frozen-setup/${path}`,
        sha256: sha256(bytes),
      },
    ]),
);

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
    codexLauncherExecutable: environment.codex.launcherExecutable,
    codexLauncherSha256: environment.codex.launcherSha256,
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
  setupSnapshots,
  prerequisitePreflight: {
    status: "passed",
    harnessRevision,
  },
};

await mkdir(dirname(output), { recursive: true });
await writeFile(output, `${JSON.stringify(lock, null, 2)}\n`, "utf8");
process.stdout.write(`${output}\n`);

async function captureNpmLocks(worktree, stagedPaths, snapshots) {
  for (const path of stagedPaths.filter((candidate) =>
    candidate.endsWith("package-lock.json"),
  )) {
    const bytes = await readFile(join(worktree, path));
    const existing = snapshots.get(path);
    if (existing !== undefined && !existing.equals(bytes)) {
      throw new Error(`npm lock changed across execution bases: ${path}`);
    }
    snapshots.set(path, bytes);
  }
}

async function writeSetupSnapshots(root, snapshots) {
  for (const [path, bytes] of snapshots) {
    const destination = join(root, path);
    await mkdir(dirname(destination), { recursive: true });
    await writeFile(destination, bytes);
  }
}

async function verifyFrozenTaskBases({
  sourceRepo,
  harnessRevision,
  prerequisiteManifest,
  taskManifest,
  expectedTaskBases,
  frozenSetupRoot,
}) {
  const root = await mkdtemp(join(tmpdir(), "uiko-g0-lock-verification-"));
  try {
    for (const arm of applicableArms(taskManifest)) {
      for (const task of applicableTasks(taskManifest, arm)) {
        const worktree = join(root, `${arm}-${task.id}`);
        addWorktree(sourceRepo, worktree, harnessRevision);
        try {
          const prepared = prepareExecutionBase({
            repoRoot: worktree,
            arm,
            taskId: task.id,
            prerequisiteManifest,
            frozenSetupRoot,
          });
          const expected = expectedTaskBases[arm][task.id];
          if (prepared.baseRevision !== expected) {
            throw new Error(
              `frozen setup cannot reproduce ${arm}/${task.id}: expected ${expected}, got ${prepared.baseRevision}`,
            );
          }
        } finally {
          removeWorktree(sourceRepo, worktree);
        }
      }
    }
  } finally {
    await rm(root, { recursive: true, force: true });
  }
}

function assertFrozenEnvironment(actual, adapterLock, harnessPackage) {
  const expectedNode = `v${harnessPackage.engines.node}`;
  const expectedNpm = harnessPackage.packageManager.split("@").at(-1);

  if (actual.node !== expectedNode) {
    throw new Error(
      `Node mismatch: expected ${expectedNode}, got ${actual.node}`,
    );
  }
  if (actual.npm !== expectedNpm) {
    throw new Error(
      `npm mismatch: expected ${expectedNpm}, got ${actual.npm}`,
    );
  }
  if (!actual.codex.versionOutput.includes(adapterLock.codexCli.version)) {
    throw new Error(
      `Codex version mismatch: expected ${adapterLock.codexCli.version}, got ${actual.codex.versionOutput}`,
    );
  }
}

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

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
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
