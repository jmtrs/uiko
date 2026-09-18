import { spawnSync } from "node:child_process";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { parseArgs } from "node:util";

import { prepareExecutionBase } from "../prepare-base.mjs";

const CASES = [
  { arm: "C_UIKO", successorTask: "G0-D03", predecessorTask: "G0-D01" },
  { arm: "C_UIKO", successorTask: "G0-D04", predecessorTask: "G0-D02" },
  { arm: "C_UIKO", successorTask: "G0-D05", predecessorTask: "G0-D03" },
  { arm: "C_UIKO", successorTask: "G0-D06", predecessorTask: "G0-D02" },
  { arm: "B_FULL", successorTask: "G0-D03", predecessorTask: "G0-D01" },
  { arm: "B_FULL", successorTask: "G0-D04", predecessorTask: "G0-D02" },
  { arm: "B_FULL", successorTask: "G0-D05", predecessorTask: "G0-D03" },
  { arm: "B_FULL", successorTask: "G0-D06", predecessorTask: "G0-D02" },
  {
    arm: "R_RENDER_ONLY",
    successorTask: "G0-D05",
    predecessorTask: "G0-D03",
  },
  {
    arm: "T_AUTHORING",
    successorTask: "G0-D05",
    predecessorTask: "G0-D03",
  },
];

const { values } = parseArgs({
  options: {
    repo: { type: "string", default: resolve(".") },
  },
});

const sourceRepo = resolve(values.repo);
const revision = capture("git", ["rev-parse", "HEAD"], sourceRepo).trim();
const root = await mkdtemp(join(tmpdir(), "uiko-g0-prerequisite-preflight-"));
const results = [];

try {
  for (const item of CASES) {
    const label = `${item.arm}-${item.successorTask}`;
    const worktree = join(root, label);
    addWorktree(sourceRepo, worktree, revision);

    try {
      const manifest = JSON.parse(
        await readFile(
          join(worktree, "experiments/g0/prerequisite-bases.json"),
          "utf8",
        ),
      );
      const prepared = prepareExecutionBase({
        repoRoot: worktree,
        arm: item.arm,
        taskId: item.successorTask,
        prerequisiteManifest: manifest,
      });

      const args = [
        join(worktree, "experiments/g0/harness/run-acceptance.mjs"),
        "--arm",
        item.arm,
        "--task",
        item.predecessorTask,
        "--repo",
        worktree,
      ];
      args.push("--preflight");

      const acceptance = spawnSync(process.execPath, args, {
        cwd: worktree,
        encoding: "utf8",
        maxBuffer: 16 * 1024 * 1024,
      });
      if (acceptance.status !== 0) {
        throw new Error(
          `prerequisite acceptance failed for ${label}:\n${acceptance.stdout || ""}\n${acceptance.stderr || ""}`,
        );
      }

      results.push({
        arm: item.arm,
        successorTask: item.successorTask,
        predecessorTask: item.predecessorTask,
        prerequisiteRevision: prepared.prerequisiteRevision,
        executionBaseRevision: prepared.baseRevision,
      });
    } finally {
      removeWorktree(sourceRepo, worktree);
    }
  }

  process.stdout.write(
    `${JSON.stringify(
      {
        schemaVersion: 1,
        status: "passed",
        harnessRevision: revision,
        cases: results,
      },
      null,
      2,
    )}\n`,
  );
} finally {
  await rm(root, { recursive: true, force: true });
}

function addWorktree(repo, worktree, revision) {
  const result = spawnSync(
    "git",
    ["worktree", "add", "--detach", worktree, revision],
    { cwd: repo, encoding: "utf8" },
  );
  if (result.status !== 0) {
    throw new Error(
      `cannot create preflight worktree: ${result.stderr || result.stdout}`,
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
      `cannot remove preflight worktree: ${result.stderr || result.stdout}`,
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
