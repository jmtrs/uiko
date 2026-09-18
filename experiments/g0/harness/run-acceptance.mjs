import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { parseArgs } from "node:util";
import { fileURLToPath } from "node:url";

import { chromium } from "@playwright/test";

import { runTaskAcceptance } from "./acceptance.mjs";
import { launchArm } from "./launch.mjs";

const EXPECTED_CRITERIA = {
  "G0-D01": 4,
  "G0-D02": 4,
  "G0-D03": 4,
  "G0-D04": 4,
  "G0-D05": 5,
  "G0-D06": 4,
};

class Reporter {
  constructor(root, trace, arm, taskId) {
    this.root = root;
    this.trace = trace;
    this.arm = arm;
    this.taskId = taskId;
    this.criteria = [];
    this.fatalError = null;
    this.preflightDiagnostics = null;
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
      this.emit({ kind: "acceptance", criterionIndex: index, passed: true, evidence });
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

  complete() {
    const expected = EXPECTED_CRITERIA[this.taskId];
    return (
      this.fatalError === null &&
      this.criteria.length === expected &&
      this.criteria.every((criterion) => criterion.passed)
    );
  }

  validation() {
    this.emit({
      kind: "validation",
      command: `g0-acceptance ${this.taskId} ${this.arm}`,
      success: this.complete(),
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
      success: this.complete(),
      fatalError: this.fatalError,
      ...(this.preflightDiagnostics === null
        ? {}
        : { preflightDiagnostics: this.preflightDiagnostics }),
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
  if (last.kind === "run_end") {
    throw new Error("acceptance must run before the run_end trace event");
  }
  return {
    runId: last.runId,
    nextSequence: last.sequence + 1,
  };
}

async function main() {
  const { values } = parseArgs({
    options: {
      arm: { type: "string" },
      task: { type: "string" },
      trace: { type: "string" },
      repo: { type: "string" },
      preflight: { type: "boolean", default: false },
    },
  });

  if (values.arm === undefined || values.task === undefined) {
    throw new Error(
      "usage: node run-acceptance.mjs --arm B_FULL|C_UIKO|R_RENDER_ONLY|T_AUTHORING --task G0-D01 [--trace TRACE]",
    );
  }
  if (!["B_FULL", "C_UIKO", "R_RENDER_ONLY", "T_AUTHORING"].includes(values.arm)) {
    throw new Error(`unsupported G0 arm ${values.arm}`);
  }
  const unregisteredProbeTask =
    ["R_RENDER_ONLY", "T_AUTHORING"].includes(values.arm) &&
    !["G0-D01", "G0-D02", "G0-D05"].includes(values.task);
  const allowedPrerequisiteCheck =
    values.preflight === true && values.task === "G0-D03";
  if (unregisteredProbeTask && !allowedPrerequisiteCheck) {
    throw new Error(`task ${values.task} is not registered for ${values.arm}`);
  }
  if (!(values.task in EXPECTED_CRITERIA)) {
    throw new Error(`invalid task id ${values.task}`);
  }

  const here = dirname(fileURLToPath(import.meta.url));
  const repoRoot = resolve(values.repo ?? resolve(here, "../../.."));
  const tracePath = values.trace === undefined ? undefined : resolve(values.trace);
  const reporter = new Reporter(repoRoot, tracePath, values.arm, values.task);

  let session;
  let browser;
  let page;

  try {
    session = await launchArm(repoRoot, values.arm);
    browser = await chromium.launch({ headless: true });
    const context = await browser.newContext();
    page = await context.newPage();

    await runTaskAcceptance({
      repoRoot,
      arm: values.arm,
      taskId: values.task,
      baseUrl: session.baseUrl,
      fixtureUrl: session.fixtureUrl,
      page,
      reporter,
    });
  } catch (error) {
    reporter.fatalError = error instanceof Error ? error.message : String(error);
    if (values.preflight === true && page !== undefined) {
      reporter.preflightDiagnostics = await collectPreflightDiagnostics(page);
    }
  } finally {
    await browser?.close().catch(() => {});
    await session?.stop().catch(() => {});
  }

  reporter.validation();
  reporter.printSummary();

  if (!reporter.complete()) {
    process.exitCode = 1;
  }
}

async function collectPreflightDiagnostics(page) {
  try {
    const host = page.locator("[data-uiko-host]").first();
    const queryError = page.locator("[data-uiko-query-error]").first();
    return {
      url: page.url(),
      bodyText: (await page.locator("body").innerText()).slice(0, 4_000),
      hostState: (await host.count()) > 0
        ? await host.getAttribute("data-uiko-host")
        : null,
      route: (await host.count()) > 0
        ? await host.getAttribute("data-uiko-route")
        : null,
      queryError: (await queryError.count()) > 0
        ? await queryError.innerText()
        : null,
    };
  } catch (error) {
    return {
      diagnosticError: error instanceof Error ? error.message : String(error),
    };
  }
}

await main();
