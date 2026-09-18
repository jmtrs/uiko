#!/usr/bin/env node
// G0 manual pilot driver: one paired (arm, task) execution with Claude Code
// via the local claude-sub-zai GLM backend. Pilot-grade only: no M5 trace,
// no uiko-g0-runner, no lock enforcement. Produces the raw transcript,
// acceptance outcome and coarse diff stats needed to decide whether the
// full measured G0 adapter is worth building.
//
// Usage:
//   node experiments/g0/claude/pilot.mjs --arm B_FULL --task G0-D01
//   node experiments/g0/claude/pilot.mjs --arm C_UIKO --task G0-D01

import { spawn, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";
import { prepareExecutionBase } from "../prepare-base.mjs";

const here = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(here, "..", "..", "..");

const args = process.argv.slice(2);
function readFlag(name, fallback = null) {
  const i = args.indexOf(`--${name}`);
  if (i === -1) return fallback;
  const v = args[i + 1];
  if (!v || v.startsWith("--")) throw new Error(`--${name} requires a value`);
  return v;
}
const arm = readFlag("arm");
const taskId = readFlag("task", "G0-D01");
if (!["B_FULL", "C_UIKO", "R_RENDER_ONLY", "T_AUTHORING"].includes(arm)) {
  throw new Error(`--arm is required (B_FULL|C_UIKO|...)`);
}

const runId = `pilot-${taskId}-${arm}`;
const outDir = path.join("/tmp", "uiko-pilot", runId);
const worktree = `${outDir}-worktree`;
for (const p of [outDir, worktree]) {
  if (fs.existsSync(p)) throw new Error(`refusing to overwrite existing ${p}`);
}
fs.mkdirSync(outDir, { recursive: true });

function run(cmd, cwd = repoRoot, opts = {}) {
  const r = spawnSync(cmd, {
    cwd,
    encoding: "utf8",
    ...opts,
    shell: false,
  });
  return r;
}
function must(cmd, cwd, opts = {}) {
  const r = run(cmd, cwd, opts);
  if (r.status !== 0) {
    throw new Error(
      `${cmd.join(" ")} failed (${r.status}): ${r.stderr || r.stdout}`
    );
  }
  return r;
}

// --- 1. clean repo, worktree at HEAD, deterministic execution base ----------
const status = run(["git", "status", "--porcelain", "--untracked-files=all"]);
if (status.status !== 0 || status.stdout.trim() !== "") {
  throw new Error(`repository must be clean before a pilot run:
${status.stdout}`);
}
must(["git", "worktree", "add", "--detach", worktree, "HEAD"]);
const harnessRevision = run(["git", "rev-parse", "HEAD"]).stdout.trim();

const prerequisiteManifest = JSON.parse(
  fs.readFileSync(path.join(worktree, "experiments", "g0", "prerequisite-bases.json"), "utf8")
);
console.log(`[pilot] preparing execution base (${arm}/${taskId}) — this installs deps, be patient…`);
const startedBaseMs = Date.now();
const base = await prepareExecutionBase({
  repoRoot: worktree,
  arm,
  taskId,
  prerequisiteManifest,
});
console.log(
  `[pilot] base ready in ${((Date.now() - startedBaseMs) / 60000).toFixed(1)} min — prerequisite ${base.prerequisiteRevision.slice(0, 10)} base ${base.baseRevision.slice(0, 10)}`
);

// --- 2. frozen task prompt ---------------------------------------------------
const manifest = JSON.parse(
  fs.readFileSync(path.join(worktree, "experiments", "tasks", "dev-manifest.json"), "utf8")
);
const task = manifest.tasks.find((t) => t.id === taskId);
if (!task) throw new Error(`unknown task ${taskId}`);
if (!manifest.primaryArms.includes(arm)) {
  throw new Error(`${arm} is not a primary arm of this manifest`);
}

// --- 3. claude-sub-zai environment (scrubbed, hermetic) -----------------------
const SCRUB = [
  "CLAUDECODE",
  "CLAUDE_CODE_CHILD_SESSION",
  "CLAUDE_CODE_SSE_PORT",
  "CLAUDE_CODE_ENTRYPOINT",
];
const keyFile = path.join(process.env.HOME, ".chelper", "coding_plan_key.txt");
const authToken = fs.readFileSync(keyFile, "utf8").trim();
if (!authToken) throw new Error(`empty GLM plan key at ${keyFile}`);

function buildClaudeEnv() {
  const env = { ...process.env };
  for (const k of SCRUB) delete env[k];
  env.ANTHROPIC_BASE_URL = "https://api.z.ai/api/anthropic";
  env.ANTHROPIC_AUTH_TOKEN = authToken;
  env.ANTHROPIC_MODEL = "glm-5.2[1m]";
  env.ANTHROPIC_DEFAULT_OPUS_MODEL = "glm-5.2[1m]";
  env.ANTHROPIC_DEFAULT_SONNET_MODEL = "glm-4.7";
  env.ANTHROPIC_DEFAULT_HAIKU_MODEL = "glm-4.5-air";
  env.API_TIMEOUT_MS = "3000000";
  env.CLAUDE_CODE_AUTO_COMPACT_WINDOW = "200000";
  env.CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC = "1";
  env.CLAUDE_CODE_FORCE_SESSION_PERSISTENCE = "1";
  env.CLAUDE_CONFIG_DIR = path.join(outDir, "claude-home");
  return env;
}

function buildClaudeArgs() {
  return [
    "--bare",
    "-p",
    "--output-format",
    "stream-json",
    "--verbose",
    "--permission-mode",
    "bypassPermissions",
    "--model",
    "glm-5.2[1m]",
  ];
}

// --- 4. one agent turn --------------------------------------------------------
const promptPath = path.join(outDir, "turn-01.prompt.txt");
fs.writeFileSync(promptPath, task.prompt);
const rawJsonlPath = path.join(outDir, "turn-01.claude.jsonl");
const stderrPath = path.join(outDir, "turn-01.claude.stderr.log");

console.log(`[pilot] launching claude on ${arm}/${taskId} (prompt frozen in ${promptPath})`);
const agentStartMs = Date.now();
const turn = await new Promise((resolve) => {
  const child = spawn("claude", [...buildClaudeArgs(), task.prompt], {
    cwd: worktree,
    env: buildClaudeEnv(),
    stdio: ["ignore", "pipe", "pipe"],
  });
  const out = fs.createWriteStream(rawJsonlPath);
  const err = fs.createWriteStream(stderrPath);
  child.stdout.pipe(out);
  child.stderr.pipe(err);
  const timeout = setTimeout(() => {
    child.kill("SIGTERM");
    resolve({ exitCode: child.exitCode ?? null, timedOut: true });
  }, 30 * 60 * 1000);
  timeout.unref();
  child.on("close", (code) => {
    clearTimeout(timeout);
    resolve({ exitCode: code, timedOut: false });
  });
});
const agentWallMs = Date.now() - agentStartMs;

// parse the transcript for usage/result
let resultEvent = null;
let parseErrors = 0;
let observedModels = new Set();
for (const line of fs.readFileSync(rawJsonlPath, "utf8").split("\n")) {
  if (!line.trim()) continue;
  try {
    const e = JSON.parse(line);
    if (e.type === "result") resultEvent = e;
    if (e.type === "assistant" && e.message?.model) observedModels.add(e.message.model);
  } catch {
    parseErrors += 1;
  }
}
console.log(
  `[pilot] claude exit=${turn.exitCode} timedOut=${turn.timedOut} subtype=${resultEvent?.subtype ?? "n/a"} numTurns=${resultEvent?.num_turns ?? "n/a"} wall=${(agentWallMs / 60000).toFixed(1)} min models=${[...observedModels].join(",")}`
);

// --- 5. shared acceptance harness ---------------------------------------------
console.log(`[pilot] running acceptance harness…`);
const acc = run(
  [
    process.execPath,
    path.join(worktree, "experiments", "g0", "harness", "run-acceptance.mjs"),
    "--arm",
    arm,
    "--task",
    taskId,
    "--repo",
    worktree,
  ],
  worktree,
  { timeout: 10 * 60 * 1000 }
);
fs.writeFileSync(path.join(outDir, "acceptance.stdout.log"), acc.stdout ?? "");
fs.writeFileSync(path.join(outDir, "acceptance.stderr.log"), acc.stderr ?? "");
const accepted = acc.status === 0;
console.log(`[pilot] acceptance exit=${acc.status} -> ${accepted ? "PASSED" : "FAILED"}`);
try {
  const tail = (acc.stdout ?? "").trim().split("\n").slice(-3).join("\n");
  if (tail) console.log(tail);
} catch {}

// --- 6. coarse diff stats (pilot grade) ----------------------------------------
const APP_PREFIX = {
  B_FULL: ["baselines/b-full/src/"],
  C_UIKO: ["fixtures/support-console/"],
}[arm];
const APP_EXCLUDE = {
  B_FULL: ["baselines/b-full/src/api/generated/"],
  C_UIKO: ["fixtures/support-console/integrations/"],
}[arm];
function classify(p) {
  if (APP_EXCLUDE.some((x) => p.startsWith(x))) return "excluded";
  if (APP_PREFIX.some((x) => p.startsWith(x))) return "application";
  return "other";
}
const diff = run(["git", "diff", "--numstat", base.baseRevision], worktree);
const untracked = run(
  ["git", "ls-files", "-o", "--exclude-standard"],
  worktree
);
const tokensOf = (s) => (s.match(/[\p{L}\p{N}_]+|[^\s]/gu) ?? []).length;

const perFile = [];
for (const line of (diff.stdout ?? "").trim().split("\n")) {
  if (!line) continue;
  const [add, del, file] = line.split("\t");
  perFile.push({ file, cls: classify(file), added: Number(add), deleted: Number(del) });
}
for (const f of (untracked.stdout ?? "").trim().split("\n")) {
  if (!f) continue;
  const full = path.join(worktree, f);
  if (!fs.statSync(full).isFile()) continue;
  const text = fs.readFileSync(full, "utf8");
  perFile.push({ file: f, cls: classify(f), added: text.split("\n").length, deleted: 0 });
}
// token count of added application text
let appAddedTokens = 0;
for (const { file, cls } of perFile) {
  if (cls !== "application") continue;
  const patch = run(
    ["git", "diff", base.baseRevision, "--", file],
    worktree
  );
  if (patch.status === 0 && (patch.stdout ?? "").trim()) {
    const addedLines = patch.stdout.split("\n").filter((l) => l.startsWith("+") && !l.startsWith("+++"));
    appAddedTokens += tokensOf(addedLines.join("\n"));
  } else {
    // untracked: whole file
    appAddedTokens += tokensOf(fs.readFileSync(path.join(worktree, file), "utf8"));
  }
}

const summary = {
  schemaVersion: "pilot-1",
  runId,
  arm,
  taskId,
  harnessRevision,
  baseRevision: base.baseRevision,
  agent: {
    tool: "claude-code/2.1.198 (bare, stream-json)",
    model: "glm-5.2[1m]",
    observedModels: [...observedModels],
    exitCode: turn.exitCode,
    timedOut: turn.timedOut,
    resultSubtype: resultEvent?.subtype ?? null,
    numTurns: resultEvent?.num_turns ?? null,
    usage: resultEvent?.usage
      ? {
          inputTokens: resultEvent.usage.input_tokens,
          outputTokens: resultEvent.usage.output_tokens,
          cacheRead: resultEvent.usage.cache_read_input_tokens,
        }
      : null,
    parseErrors,
  },
  acceptance: { exitCode: acc.status, accepted },
  agentWallMs,
  files: perFile,
  appAddedTokens,
};
fs.writeFileSync(path.join(outDir, "summary.json"), JSON.stringify(summary, null, 2));
console.log(`[pilot] summary:`);
console.log(
  JSON.stringify(
    { ...summary, files: `${perFile.length} changed (see summary.json)` },
    null,
    2
  )
);
console.log(`[pilot] artifacts in ${outDir} (worktree kept at ${worktree})`);
