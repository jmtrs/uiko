import { spawnSync } from "node:child_process";
import { readFile, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { parseArgs } from "node:util";

import { TraceWriter } from "./trace-adapter.mjs";

const CATEGORIES = new Set([
  "LOCAL",
  "WIRING",
  "COHERENCE",
  "VISUAL_FIT",
  "ENVIRONMENT",
]);

async function main() {
  const { values } = parseArgs({
    options: {
      pending: { type: "string" },
      review: { type: "string" },
    },
  });

  const pendingPath = required(values.pending, "--pending");
  const reviewPath = required(values.review, "--review");
  const pending = await readJson(resolve(pendingPath));
  const review = await readJson(resolve(reviewPath));

  validateReview(pending, review);

  const runnerPath = resolve(pending.worktree, "target/debug/uiko-g0-runner");
  const writer = new TraceWriter({
    repoRoot: pending.worktree,
    tracePath: pending.tracePath,
    runnerPath,
    runId: pending.runId,
    taskId: pending.taskId,
    arm: pending.arm,
  });

  for (const repair of [...review.repairs].sort((a, b) => a.iteration - b.iteration)) {
    await writer.append({
      kind: "repair",
      iteration: repair.iteration,
      category: repair.category,
      reason: repair.reason,
      evidence: repair.evidence,
      sourcePaths: repair.sourcePaths ?? [],
    });
  }

  const endedAt = new Date();
  await writer.append({
    kind: "run_end",
    endedAt: endedAt.toISOString(),
    endedUnixMs: endedAt.getTime(),
    outcome: pending.candidateOutcome,
    finalRevision: null,
  });

  const resultPath = resolve(pending.resultsDir, "result.json");
  const aggregate = spawnSync(
    runnerPath,
    [
      "aggregate",
      "--repo",
      pending.worktree,
      "--trace",
      pending.tracePath,
      "--path-policy",
      resolve(pending.worktree, "experiments/g0/path-policy.json"),
      "--output",
      resultPath,
    ],
    { cwd: pending.worktree, encoding: "utf8" },
  );
  if (aggregate.status !== 0) {
    throw new Error(
      `G0 aggregation failed: ${aggregate.stderr || aggregate.stdout || "unknown error"}`,
    );
  }

  const marker = {
    schemaVersion: 1,
    status: "finalized",
    runId: pending.runId,
    result: resultPath,
    repairReviewer: review.reviewer,
    finalizedAt: endedAt.toISOString(),
  };
  await writeFile(
    resolve(pending.resultsDir, "finalized.json"),
    `${JSON.stringify(marker, null, 2)}\n`,
    "utf8",
  );

  process.stdout.write(`${JSON.stringify(marker, null, 2)}\n`);
}

export function validateReview(pending, review) {
  if (pending.status !== "awaiting-repair-review") {
    throw new Error("pending run is not awaiting repair review");
  }
  if (!Number.isInteger(pending.turns) || pending.turns < 1) {
    throw new Error("pending run must record a positive turn count");
  }
  if (review.schemaVersion !== 1 || review.status !== "reviewed") {
    throw new Error("repair review must be schemaVersion=1 and status=reviewed");
  }
  if (review.runId !== pending.runId) {
    throw new Error("repair review runId does not match pending run");
  }
  if (typeof review.reviewer !== "string" || review.reviewer.length === 0) {
    throw new Error("repair review must record a reviewer");
  }
  if (!Array.isArray(review.repairs)) {
    throw new Error("repair review repairs must be an array");
  }

  const iterations = new Set();
  for (const repair of review.repairs) {
    if (!Number.isInteger(repair.iteration) || repair.iteration < 1) {
      throw new Error("repair iteration must be a positive integer");
    }
    if (repair.iteration > pending.turns - 1) {
      throw new Error(
        `repair iteration ${repair.iteration} exceeds the ${Math.max(0, pending.turns - 1)} possible repair iteration(s)`,
      );
    }
    if (iterations.has(repair.iteration)) {
      throw new Error(`duplicate repair iteration ${repair.iteration}`);
    }
    iterations.add(repair.iteration);
    if (!CATEGORIES.has(repair.category)) {
      throw new Error(`invalid repair category ${repair.category}`);
    }
    if (typeof repair.reason !== "string" || repair.reason.length === 0) {
      throw new Error("repair reason must be non-empty");
    }
    if (typeof repair.evidence !== "string" || repair.evidence.length === 0) {
      throw new Error("repair evidence must be non-empty");
    }
    if (
      repair.sourcePaths !== undefined &&
      (!Array.isArray(repair.sourcePaths) ||
        repair.sourcePaths.some((path) => typeof path !== "string"))
    ) {
      throw new Error("repair sourcePaths must be an array of strings");
    }
  }
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

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href
) {
  await main();
}
