#!/usr/bin/env node
// G1 seeded-review pilot driver.
//
// Materializes each corpus case (base + overlay), runs `uiko diff base AFTER`,
// and either:
//   --check   validate every case against cases.json ground truth + mechanical
//             completeness (exit 0 pass, 1 fail). CI-safe.
//   --packet  emit the anonymized reviewer packet: shuffled human diffs with no
//             answer key, one file per case, plus a stable packet manifest.
// Default (no flag) prints a per-case summary table for a quick local look.
//
// Env: UIKO_BIN overrides the compiled CLI path (default ./target/debug/uiko).

import { execFileSync } from "node:child_process";
import { cpSync, mkdtempSync, mkdirSync, writeFileSync, rmSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { readFileSync, readdirSync, statSync } from "node:fs";

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(here, "..", "..");
const uikoBin = process.env.UIKO_BIN ?? join(repoRoot, "target", "debug", "uiko");
const spec = JSON.parse(readFileSync(join(here, "cases.json"), "utf8"));
const baseDir = join(here, spec.base);

function overlayFiles(dir, prefix = "") {
  const out = [];
  for (const entry of readdirSync(dir)) {
    const abs = join(dir, entry);
    const rel = prefix ? join(prefix, entry) : entry;
    if (statSync(abs).isDirectory()) out.push(...overlayFiles(abs, rel));
    else out.push(rel);
  }
  return out;
}

// Materialize AFTER = copy of base with the case overlay applied, then run
// `uiko diff base AFTER --format json`. Returns { status, rows }.
function diffCase(id) {
  const overlayDir = join(here, "corpus", id);
  if (!existsSync(overlayDir)) throw new Error(`missing overlay: corpus/${id}`);
  const after = mkdtempSync(join(tmpdir(), `g1-${id}-`));
  try {
    cpSync(baseDir, after, { recursive: true });
    for (const rel of overlayFiles(overlayDir)) {
      const dst = join(after, rel);
      mkdirSync(dirname(dst), { recursive: true });
      cpSync(join(overlayDir, rel), dst);
    }
    let stdout, code;
    try {
      stdout = execFileSync(uikoBin, ["diff", baseDir, after, "--format", "json"], {
        encoding: "utf8",
      });
      code = 0;
    } catch (err) {
      stdout = err.stdout?.toString() ?? "";
      code = err.status ?? -1;
    }
    // exit 2 = usage/compile failure: the diff never ran, ground truth is moot.
    if (code === 2) return { status: "compile-error", rows: [], raw: stdout, code };
    const parsed = JSON.parse(stdout);
    return { status: parsed.status, rows: parsed.rows, code };
  } finally {
    rmSync(after, { recursive: true, force: true });
  }
}

function humanDiff(id) {
  const overlayDir = join(here, "corpus", id);
  const after = mkdtempSync(join(tmpdir(), `g1-${id}-`));
  try {
    cpSync(baseDir, after, { recursive: true });
    for (const rel of overlayFiles(overlayDir)) {
      const dst = join(after, rel);
      mkdirSync(dirname(dst), { recursive: true });
      cpSync(join(overlayDir, rel), dst);
    }
    try {
      return execFileSync(uikoBin, ["diff", baseDir, after], { encoding: "utf8" });
    } catch (err) {
      return err.stdout?.toString() ?? "";
    }
  } finally {
    rmSync(after, { recursive: true, force: true });
  }
}

function kindsOf(rows) {
  return rows.map((r) => r.kind).sort();
}

function check() {
  let failures = 0;
  for (const c of spec.cases) {
    const { status, rows } = diffCase(c.id);
    const got = kindsOf(rows);
    const want = [...c.expectedKinds].sort();
    const problems = [];

    if (status === "compile-error") problems.push("compiled with error (exit 2) — case does not diff");
    if (c.kind === "clean" && rows.length !== 0)
      problems.push(`clean case produced rows: ${got.join(", ")}`);
    if (c.kind === "seeded") {
      if (rows.length === 0) problems.push("seeded case produced no rows (mechanical incompleteness)");
      // Mechanical completeness: every expected kind must be present. Extra
      // rows caused by one root change (e.g. a contract edit touching two
      // queries) are allowed; the expected kinds must be a subset.
      for (const k of want) if (!got.includes(k)) problems.push(`missing expected kind: ${k}`);
    }

    const mark = problems.length ? "FAIL" : "ok";
    if (problems.length) failures++;
    console.log(`[${mark}] ${c.id.padEnd(28)} want=[${want.join(",")}] got=[${got.join(",")}]`);
    for (const p of problems) console.log(`        - ${p}`);
  }
  console.log(`\n${spec.cases.length} cases, ${failures} failing.`);
  process.exit(failures ? 1 : 0);
}

// Deterministic shuffle (seeded) so the packet order hides clean/seeded grouping
// but is reproducible for scoring.
function shuffled(ids, seed = 0x9e3779b9) {
  const arr = [...ids];
  let s = seed >>> 0;
  const rand = () => ((s = (s * 1664525 + 1013904223) >>> 0), s / 0x100000000);
  for (let i = arr.length - 1; i > 0; i--) {
    const j = Math.floor(rand() * (i + 1));
    [arr[i], arr[j]] = [arr[j], arr[i]];
  }
  return arr;
}

function packet() {
  const outDir = join(here, "packet");
  rmSync(outDir, { recursive: true, force: true });
  mkdirSync(outDir, { recursive: true });
  const order = shuffled(spec.cases.map((c) => c.id));
  const manifest = [];
  order.forEach((id, i) => {
    const label = `R${String(i + 1).padStart(2, "0")}`;
    const diff = humanDiff(id);
    writeFileSync(join(outDir, `${label}.diff.txt`), diff);
    manifest.push({ label, caseId: id });
  });
  writeFileSync(join(outDir, "packet-manifest.json"), JSON.stringify({ order: manifest }, null, 2) + "\n");
  console.log(`Wrote ${order.length} anonymized diffs + packet-manifest.json to experiments/g1/packet/`);
  console.log("Give reviewers only the R##.diff.txt files. Keep packet-manifest.json + cases.json as the key.");
}

function summary() {
  for (const c of spec.cases) {
    const { status, rows } = diffCase(c.id);
    console.log(`${c.id.padEnd(28)} ${status.padEnd(14)} rows=${rows.length} [${kindsOf(rows).join(", ")}]`);
  }
}

// Self-contained reviewer handout: one Markdown file with instructions, all 13
// anonymized diffs inline, and a blank answer table. A reviewer needs nothing
// but this file; they return the filled table. Regenerates the packet first so
// the diffs and manifest stay in sync.
function handout() {
  packet();
  const order = JSON.parse(readFileSync(join(here, "packet", "packet-manifest.json"), "utf8")).order;
  const L = [];
  L.push("# uiko plan-diff review\n");
  L.push("You are reviewing compiled-plan diffs. For each item below, decide **from the diff alone**");
  L.push("whether it hides a **consequential change** (something that changes behavior, data, access");
  L.push("or contract) or is a **no-op** (cosmetic / no real change).\n");
  L.push("For each `R##` fill one row of the table at the end:\n");
  L.push("- **verdict**: `consequential` or `clean`");
  L.push("- **description**: if consequential, one line on what changed (else leave blank)");
  L.push("- **intelligibility**: `intelligible` if the diff was readable, `unintelligible` if its form");
  L.push("  made the change impossible to judge");
  L.push("- **note**: optional\n");
  L.push("Return the filled table (or the CSV block) to whoever sent this. Do not look at the source repo.\n");
  L.push("---\n");
  for (const { label } of order) {
    const diff = readFileSync(join(here, "packet", `${label}.diff.txt`), "utf8").trimEnd();
    L.push(`### ${label}\n`);
    L.push("```");
    L.push(diff);
    L.push("```\n");
  }
  L.push("---\n");
  L.push("## Your answers\n");
  L.push("| label | verdict | description | intelligibility | note |");
  L.push("|---|---|---|---|---|");
  for (const { label } of order) L.push(`| ${label} | | | | |`);
  L.push("\nOr fill this CSV and send it back:\n");
  L.push("```csv");
  L.push("label,verdict,description,intelligibility,note");
  for (const { label } of order) L.push(`${label},,,,`);
  L.push("```");
  const outPath = join(here, "packet", "handout.md");
  writeFileSync(outPath, L.join("\n") + "\n");
  console.log(`\nWrote self-contained reviewer handout: experiments/g1/packet/handout.md`);
  console.log("Send that one file to each reviewer. Save each reply as experiments/g1/reviewers/<name>.csv, then run score.mjs.");
}

const mode = process.argv[2];
if (mode === "--check") check();
else if (mode === "--packet") packet();
else if (mode === "--handout") handout();
else summary();
