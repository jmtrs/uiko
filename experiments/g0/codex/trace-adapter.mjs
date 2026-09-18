import { spawn, spawnSync } from "node:child_process";
import { createWriteStream } from "node:fs";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { createInterface } from "node:readline";
import { TextDecoder } from "node:util";

const decoder = new TextDecoder("utf-8", { fatal: true });

export class TraceWriter {
  constructor({ repoRoot, tracePath, runnerPath, runId, taskId, arm }) {
    this.repoRoot = repoRoot;
    this.tracePath = tracePath;
    this.runnerPath = runnerPath;
    this.runId = runId;
    this.taskId = taskId;
    this.arm = arm;
  }

  async append(payload) {
    const sequence = await nextSequence(this.tracePath);
    const event = {
      schemaVersion: 1,
      sequence,
      runId: this.runId,
      taskId: this.taskId,
      arm: this.arm,
      ...payload,
    };
    const result = spawnSync(
      this.runnerPath,
      ["append", "--trace", this.tracePath],
      {
        cwd: this.repoRoot,
        input: JSON.stringify(event),
        encoding: "utf8",
      },
    );
    if (result.status !== 0) {
      throw new Error(
        `cannot append G0 trace event: ${result.stderr || result.stdout || "unknown runner error"}`,
      );
    }
    return event;
  }
}

export class SnapshotObserver {
  constructor({ repoRoot, traceWriter }) {
    this.repoRoot = repoRoot;
    this.traceWriter = traceWriter;
    this.previous = null;
  }

  async initialize() {
    this.previous = await snapshotTextFiles(this.repoRoot);
  }

  async rebaseline() {
    this.previous = await snapshotTextFiles(this.repoRoot);
  }

  async boundary() {
    if (this.previous === null) {
      throw new Error("SnapshotObserver.initialize() must be called first");
    }
    const current = await snapshotTextFiles(this.repoRoot);
    const changes = diffSnapshots(this.previous, current);
    for (const change of changes) {
      await this.traceWriter.append({
        kind: "edit",
        path: change.path,
        operation: change.operation,
        beforeText: change.beforeText,
        afterText: change.afterText,
      });
    }
    this.previous = current;
    return changes;
  }
}

export async function runCodexTurn({
  repoRoot,
  codexBin,
  args,
  prompt,
  rawJsonlPath,
  rawStderrPath,
  observer,
  traceWriter,
}) {
  const raw = createWriteStream(rawJsonlPath, { flags: "w" });
  const stderr = createWriteStream(rawStderrPath, { flags: "w" });

  const child = spawn(codexBin, argsForPrompt(args, prompt), {
    cwd: repoRoot,
    env: process.env,
    stdio: ["ignore", "pipe", "pipe"],
  });

  child.stderr.pipe(stderr);
  const lines = createInterface({ input: child.stdout, crlfDelay: Infinity });
  let threadId = null;
  let finalMessage = null;
  let parseErrors = 0;

  for await (const line of lines) {
    raw.write(`${line}\n`);
    let event;
    try {
      event = JSON.parse(line);
    } catch {
      parseErrors += 1;
      await observer.boundary();
      continue;
    }

    await observer.boundary();

    if (event.type === "thread.started" && typeof event.thread_id === "string") {
      threadId = event.thread_id;
    }
    if (
      event.type === "item.completed" &&
      event.item?.type === "agent_message" &&
      typeof event.item.text === "string"
    ) {
      finalMessage = event.item.text;
    }

    await translateTelemetry(event, traceWriter);
  }

  const exitCode = await new Promise((resolve, reject) => {
    child.once("error", reject);
    child.once("close", (code) => resolve(code ?? 1));
  });

  await observer.boundary();
  raw.end();
  stderr.end();

  if (parseErrors > 0) {
    await traceWriter.append({
      kind: "protocol_deviation",
      code: "G0_CODEX_JSON_PARSE",
      description: `Codex emitted ${parseErrors} non-JSON stdout line(s) while --json was active`,
      impact: "material",
    });
  }

  return { exitCode, threadId, finalMessage, parseErrors };
}

export function buildCodexArgs(lock, model, reasoningEffort) {
  return [
    "exec",
    "--json",
    "--ephemeral",
    "--ignore-user-config",
    "--ignore-rules",
    "--sandbox",
    lock.execution.sandbox,
    "--model",
    model,
    "--config",
    `approval_policy="${lock.execution.approvalPolicy}"`,
    "--config",
    `model_reasoning_effort="${reasoningEffort}"`,
    "--config",
    `agents.enabled=${String(lock.execution.agentsEnabled)}`,
    "--config",
    `features.unified_exec=${String(lock.execution.unifiedExec)}`,
  ];
}

export async function snapshotTextFiles(repoRoot) {
  const result = spawnSync(
    "git",
    ["ls-files", "-co", "--exclude-standard", "-z"],
    { cwd: repoRoot, encoding: null },
  );
  if (result.status !== 0) {
    throw new Error(
      `git ls-files failed: ${result.stderr?.toString("utf8") ?? "unknown error"}`,
    );
  }

  const paths = result.stdout
    .toString("utf8")
    .split("\0")
    .filter((path) => path.length > 0);
  const snapshot = new Map();

  for (const path of paths) {
    try {
      const bytes = await readFile(resolve(repoRoot, path));
      let text;
      try {
        text = decoder.decode(bytes);
      } catch {
        continue;
      }
      snapshot.set(path.replaceAll("\\", "/"), text);
    } catch (error) {
      if (error?.code !== "ENOENT") {
        throw error;
      }
    }
  }
  return snapshot;
}

export function diffSnapshots(before, after) {
  const paths = new Set([...before.keys(), ...after.keys()]);
  const changes = [];

  for (const path of [...paths].sort()) {
    const previous = before.get(path);
    const current = after.get(path);
    if (previous === current) {
      continue;
    }
    changes.push({
      path,
      operation:
        previous === undefined
          ? "create"
          : current === undefined
            ? "delete"
            : "update",
      beforeText: previous ?? null,
      afterText: current ?? null,
    });
  }
  return changes;
}

async function translateTelemetry(event, traceWriter) {
  if (
    event.type === "item.completed" &&
    event.item?.type === "command_execution"
  ) {
    const command = commandText(event.item.command);
    const success = commandSucceeded(event.item);

    if (isRepositorySearch(command)) {
      await traceWriter.append({
        kind: "repository_search",
        query: command,
      });
    } else if (isRepositoryRead(command)) {
      await traceWriter.append({
        kind: "repository_read",
        target: command,
      });
    }

    if (isValidationCommand(command)) {
      await traceWriter.append({
        kind: "validation",
        command,
        success,
        evidence: success ? null : commandEvidence(event.item),
      });
    }
  }

  if (event.type === "turn.completed") {
    const usage = event.usage ?? event.turn?.usage;
    const input = integerUsage(usage, ["input_tokens", "inputTokens"]);
    const output = integerUsage(usage, ["output_tokens", "outputTokens"]);
    if (input !== null || output !== null) {
      await traceWriter.append({
        kind: "provider_tokens",
        inputTokens: input ?? 0,
        outputTokens: output ?? 0,
      });
    }
  }

  const message = typeof event.message === "string" ? event.message : "";
  if (
    event.type === "error" &&
    /event stream lagged|dropped .*event/i.test(message)
  ) {
    await traceWriter.append({
      kind: "protocol_deviation",
      code: "G0_CODEX_EVENT_DROP",
      description: message,
      impact: "invalidating",
    });
  }
}

function argsForPrompt(args, prompt) {
  return [...args, prompt];
}

function commandText(command) {
  if (typeof command === "string") {
    return command;
  }
  if (Array.isArray(command)) {
    return command.map(String).join(" ");
  }
  return JSON.stringify(command ?? "");
}

function commandSucceeded(item) {
  if (typeof item.exit_code === "number") {
    return item.exit_code === 0;
  }
  if (typeof item.exitCode === "number") {
    return item.exitCode === 0;
  }
  return item.status === "completed" || item.status === "success";
}

function commandEvidence(item) {
  for (const key of ["aggregated_output", "output", "stderr"]) {
    if (typeof item[key] === "string" && item[key].length > 0) {
      return item[key].slice(-4_000);
    }
  }
  return "command failed";
}

function isRepositorySearch(command) {
  return /(^|[\s;&|])(rg|grep|fd|find)([\s;&|]|$)/.test(command);
}

function isRepositoryRead(command) {
  return /(^|[\s;&|])(cat|head|tail|less|ls|tree)([\s;&|]|$)|git\s+(status|diff|show|log)\b|sed\s+-n\b/.test(
    command,
  );
}

function isValidationCommand(command) {
  return /\b(cargo\s+(test|check|clippy|build|fmt)|npm\s+(test|run\s+(typecheck|lint|build)|exec\b)|npx\s+playwright|tsc\b|vite\s+build)\b/.test(
    command,
  );
}

function integerUsage(usage, keys) {
  if (usage === null || typeof usage !== "object") {
    return null;
  }
  for (const key of keys) {
    if (Number.isInteger(usage[key]) && usage[key] >= 0) {
      return usage[key];
    }
  }
  return null;
}

async function nextSequence(tracePath) {
  try {
    const text = await readFile(tracePath, "utf8");
    const lines = text
      .split(/\r?\n/)
      .map((line) => line.trim())
      .filter(Boolean);
    if (lines.length === 0) {
      return 1;
    }
    const last = JSON.parse(lines.at(-1));
    return last.sequence + 1;
  } catch (error) {
    if (error?.code === "ENOENT") {
      return 1;
    }
    throw error;
  }
}
