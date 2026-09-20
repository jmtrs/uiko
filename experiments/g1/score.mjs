#!/usr/bin/env node
// G1 scorer. Reads the packet key + reviewer answer CSVs and computes the
// §4.2 GO/NO-GO metrics. Exit 0 = GO, 1 = NO-GO (or missing inputs).
//
//   node experiments/g1/score.mjs [reviewersDir]
//
// reviewersDir defaults to experiments/g1/reviewers/. Each *.csv there is one
// reviewer with a header row and columns:
//   label,verdict,description,intelligibility,note
//     verdict         consequential | clean
//     intelligibility intelligible  | unintelligible
// Requires experiments/g1/packet/packet-manifest.json (run.mjs --packet) and
// cases.json. Detection is verdict-based; description is printed for the human
// adjudicator but does not gate the point estimate.

import { readFileSync, readdirSync, existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const DETECTION_THRESHOLD = 0.7; // §4.2 frozen rule

function die(msg) {
  console.error(`score: ${msg}`);
  process.exit(1);
}

function parseCsv(text) {
  const rows = [];
  let field = "";
  let record = [];
  let inQuotes = false;
  for (let i = 0; i < text.length; i++) {
    const ch = text[i];
    if (inQuotes) {
      if (ch === '"') {
        if (text[i + 1] === '"') { field += '"'; i++; }
        else inQuotes = false;
      } else field += ch;
    } else if (ch === '"') inQuotes = true;
    else if (ch === ",") { record.push(field); field = ""; }
    else if (ch === "\n" || ch === "\r") {
      if (ch === "\r" && text[i + 1] === "\n") i++;
      if (field !== "" || record.length) { record.push(field); rows.push(record); record = []; field = ""; }
    } else field += ch;
  }
  if (field !== "" || record.length) { record.push(field); rows.push(record); }
  return rows;
}

function loadReviewer(path) {
  const rows = parseCsv(readFileSync(path, "utf8")).filter((r) => r.some((c) => c.trim() !== ""));
  if (!rows.length) die(`empty CSV: ${path}`);
  const header = rows[0].map((h) => h.trim().toLowerCase());
  const idx = (name) => {
    const i = header.indexOf(name);
    if (i < 0) die(`${path}: missing column "${name}"`);
    return i;
  };
  const iL = idx("label"), iV = idx("verdict"), iD = header.indexOf("description");
  const iI = idx("intelligibility");
  const answers = new Map();
  for (const r of rows.slice(1)) {
    const label = r[iL]?.trim();
    if (!label) continue;
    answers.set(label, {
      verdict: (r[iV] ?? "").trim().toLowerCase(),
      description: iD >= 0 ? (r[iD] ?? "").trim() : "",
      intelligibility: (r[iI] ?? "").trim().toLowerCase(),
    });
  }
  return answers;
}

// --- load inputs -----------------------------------------------------------
const reviewersDir = process.argv[2] ?? join(here, "reviewers");
const packetPath = join(here, "packet", "packet-manifest.json");
if (!existsSync(packetPath)) die("missing packet/packet-manifest.json — run: node run.mjs --packet");
if (!existsSync(reviewersDir)) die(`missing reviewers dir: ${reviewersDir}`);

const labelToCase = new Map(
  JSON.parse(readFileSync(packetPath, "utf8")).order.map((o) => [o.label, o.caseId]),
);
const caseSpec = new Map(JSON.parse(readFileSync(join(here, "cases.json"), "utf8")).cases.map((c) => [c.id, c]));

const csvFiles = readdirSync(reviewersDir).filter((f) => f.endsWith(".csv"));
if (!csvFiles.length) die(`no *.csv reviewer files in ${reviewersDir}`);
const reviewers = csvFiles.map((f) => ({ name: f.replace(/\.csv$/, ""), answers: loadReviewer(join(reviewersDir, f)) }));

const seededLabels = [...labelToCase].filter(([, id]) => caseSpec.get(id).kind === "seeded").map(([l]) => l);
const cleanLabels = [...labelToCase].filter(([, id]) => caseSpec.get(id).kind === "clean").map(([l]) => l);
const isConsequential = (v) => v === "consequential" || v === "yes" || v === "change";

// --- per-reviewer ----------------------------------------------------------
console.log("Reviewers\n");
for (const rv of reviewers) {
  let hit = 0, fp = 0;
  for (const l of seededLabels) if (isConsequential(rv.answers.get(l)?.verdict)) hit++;
  for (const l of cleanLabels) if (isConsequential(rv.answers.get(l)?.verdict)) fp++;
  console.log(
    `  ${rv.name.padEnd(20)} detection ${hit}/${seededLabels.length}` +
      `  false-positive ${fp}/${cleanLabels.length}`,
  );
}

// --- per-class detection ---------------------------------------------------
console.log("\nPer-class detection (reviewer-observations flagging consequential)\n");
const classStats = new Map();
let hits = 0, obs = 0;
for (const l of seededLabels) {
  const spec = caseSpec.get(labelToCase.get(l));
  const s = classStats.get(spec.class) ?? { hit: 0, total: 0 };
  for (const rv of reviewers) {
    obs++; s.total++;
    if (isConsequential(rv.answers.get(l)?.verdict)) { hits++; s.hit++; }
  }
  classStats.set(spec.class, s);
}
for (const [cls, s] of classStats)
  console.log(`  ${cls.padEnd(28)} ${s.hit}/${s.total}  (${((s.hit / s.total) * 100).toFixed(0)}%)`);

// --- unintelligibility -----------------------------------------------------
const unintByClass = new Map();
for (const l of seededLabels) {
  const spec = caseSpec.get(labelToCase.get(l));
  for (const rv of reviewers)
    if (rv.answers.get(l)?.intelligibility === "unintelligible")
      unintByClass.set(spec.class, (unintByClass.get(spec.class) ?? 0) + 1);
}
const repeatedUnint = [...unintByClass].filter(([, n]) => n >= 2);

// --- verdict ---------------------------------------------------------------
const detection = obs ? hits / obs : 0;
const passDetection = detection >= DETECTION_THRESHOLD;
const passUnint = repeatedUnint.length === 0;

console.log("\nGO / NO-GO (§4.2)\n");
console.log(`  detection point estimate  ${(detection * 100).toFixed(1)}%  (need >= ${DETECTION_THRESHOLD * 100}%)  ${passDetection ? "PASS" : "FAIL"}`);
console.log(`  no repeated unintelligible class                            ${passUnint ? "PASS" : "FAIL"}`);
if (!passUnint) for (const [cls, n] of repeatedUnint) console.log(`      - ${cls}: ${n} unintelligible marks`);
console.log("  mechanical completeness: run `node run.mjs --check` separately (must pass)\n");

const go = passDetection && passUnint;
console.log(go ? "=> GO (pending mechanical-completeness confirmation)" : "=> NO-GO");
process.exit(go ? 0 : 1);
