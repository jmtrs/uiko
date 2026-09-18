import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import { parseArgs } from "node:util";

import { chromium } from "@playwright/test";

import { runTaskAcceptance } from "./acceptance.mjs";
import { launchArm } from "./launch.mjs";

const { values } = parseArgs({
  options: {
    arm: { type: "string" },
    task: { type: "string" },
    trace: { type: "string" },
    repo: { type: "string" },
  },
});

if (values.arm === undefined || values.task === undefined) {
  throw new Error("usage: node run-acceptance.mjs --arm B_FULL|C_UIKO --task G0-D01 [--trace TRACE]");
}
if (!["B_FULL", "C_UIKO"].includes(values.arm)) {
  throw new Error(`unsupported primary arm ${values.arm}`);
}
if (!/^G0-D0[1-6]$/.test(values.task)) {
  throw new Error(`invalid task id ${values.task}`);
}

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(values.repo ?? resolve(here, "../../.."));
const tracePath = values.trace === undefined ? undefined : resolve(values.trace);
const reporter = new Reporter(repoRoot, tracePath, values.arm, values.task);

let session;
let browser;
let success = false;

try {
  session = await launchArm(repoRoot, values.arm);
  browser = await chromium.launch({ headless: true });
  const context = await browser.newContext();
  const page = await context.newPage();

  await runTaskAcceptance({
    repoRoot,
    arm: values.arm,
    taskId: values.task,
    baseUrl: session.baseUrl,
    fixtureUrl: session.fixtureUrl,
    page,
    reporter,
  });

  success = reporter.criteria.length > 0 && reporter.criteria.every((criterion) => criterion.passed);
} catch (error) {
  reporter.fatalError = error instanceof Error ? error.message : String(error);
} finally {
  await browser?.close().catch(() => {});
  await session?.stop().catch(() => {});
}

reporter.validation(success);
reporter.printSummary();

if (!success) {
  process.exitCode = 1;
}

class Reporter {
  constructor(root, trace, arm, taskId) {
    this.root = root;
    this.trace = trace;
    this.arm = arm;
    this.taskId = taskId;
    this.criteria = [];
    this.fatalError = null;
    this.identity = trace === undefined ? null : readTraceIdentity(trace, arm, taskId);
  }

  async browser(action, operation) {
    try {
      const result = await operation();
      this.emit({ kind: "browser", action, success: true, evidence: null });
      return result;
    } catch (error) {
      const evidence = error instanceof Error ? error.message : String(error);
      this.emit({ kind: "browser", action, success: false, evidence });
      throw error;
    }
  }

  async criterion(index, evidence, check) {
    try {
      await check();
      const result = { index, passed: true, evidence };
      this.criteria.push(result);
      this.emit({
        kind: "acceptance",
        criterionIndex: index,
        passed: true,
        evidence,
      });
      return true;
    } catch (error) {
      const detail = error instanceof Error ? error.message : String(error);
      const result = { index, passed: false, evidence: `${evidence}: ${detail}` };
      this.criteria.push(result);
      this.emit({
        kind: "acceptance",
        criterionIndex: index,
        passed: false,
        evidence: result.evidence,
      });
      return false;
    }
  }

  validation(success) {
    this.emit({
      kind: "validation",
      command: `g0-acceptance ${this.taskId} ${this.arm}`,
      success,
      evidence: this.fatalError,
    });
  }

  emit(payload) {
    if (this.identity === null) {
      return;
    }
    const event = {
      schemaVersion: 1,
      sequence: this.identity.nextSequence,
      runId: this.identity.runId,
      taskId: this.taskId,
      arm: this.arm,
      ...payload,
    };
    const result = spawnSync(
      "cargo",
      [
        "run",
        "--quiet",
        "-p",
        "uiko-g0-runner",
        "--",
        "append",
        "--trace",
        this.trace,
      ],
      {
        cwd: this.root,
        input: JSON.stringify(event),
        encoding: "utf8",
      },
    );
    if (result.status !== 0) {
      throw new Error(
        `cannot append G0 trace event: ${result.stderr || result.stdout || "unknown runner error"}`,
      );
    }
    this.identity.nextSequence += 1;
  }

  printSummary() {
    const result = {
      taskId: this.taskId,
      arm: this.arm,
      success: this.criteria.length > 0 && this.criteria.every((criterion) => criterion.passed),
      fatalError: this.fatalError,
      criteria: [...this.criteria].sort((left, right) => left.index - right.index),
    };
    process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
  }
}

function readTraceIdentity(path, arm, taskId) {
  const text = readFileSync(path, "utf8").trim();
  if (text.length === 0) {
    throw new Error("--trace must already contain the run_start event");
  }
  const events = text.split(/\r?\n/).map((line) => JSON.parse(line));
  const last = events.at(-1);
  if (last.arm !== arm || last.taskId !== taskId) {
    throw new Error(
      `trace identity ${last.taskId}/${last.arm} does not match ${taskId}/${arm}`,
    );
  }
  return {
    runId: last.runId,
    nextSequence: last.sequence + 1,
  };
}
