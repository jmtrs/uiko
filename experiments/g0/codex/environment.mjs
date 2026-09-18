import { createHash } from "node:crypto";
import { readFile, realpath } from "node:fs/promises";
import { resolve } from "node:path";
import { spawnSync } from "node:child_process";

export async function inspectEnvironment(repoRoot, codexBin = "codex") {
  const codexVersion = runCapture(codexBin, ["--version"], repoRoot).trim();
  const codexPathRaw = runCapture("which", [codexBin], repoRoot).trim();
  const codexPath = await realpath(codexPathRaw);
  const codexBytes = await readFile(codexPath);
  const codexSha256 = createHash("sha256").update(codexBytes).digest("hex");

  return {
    os: process.platform,
    arch: process.arch,
    node: process.version,
    npm: runCapture("npm", ["--version"], repoRoot).trim(),
    rustc: runCapture("rustc", ["--version"], repoRoot).trim(),
    browser: browserVersion(repoRoot),
    codex: {
      versionOutput: codexVersion,
      executable: codexPath,
      sha256: codexSha256,
    },
  };
}

export function assertEnvironmentMatches(actual, lock, adapterLock) {
  if (!actual.codex.versionOutput.includes(adapterLock.codexCli.version)) {
    throw new Error(
      `Codex version mismatch: expected ${adapterLock.codexCli.version}, got ${actual.codex.versionOutput}`,
    );
  }

  const expected = lock.environment;
  for (const key of ["os", "arch", "node", "npm", "rustc", "browser"]) {
    if (expected[key] !== actual[key]) {
      throw new Error(
        `environment mismatch for ${key}: expected ${expected[key]}, got ${actual[key]}`,
      );
    }
  }

  if (lock.agent.codexExecutableSha256 !== actual.codex.sha256) {
    throw new Error(
      `Codex executable mismatch: expected ${lock.agent.codexExecutableSha256}, got ${actual.codex.sha256}`,
    );
  }
}

export function runCapture(command, args, cwd) {
  const result = spawnSync(command, args, {
    cwd,
    encoding: "utf8",
  });
  if (result.status !== 0) {
    throw new Error(
      `${command} ${args.join(" ")} failed: ${result.stderr || result.stdout}`,
    );
  }
  return result.stdout;
}

function browserVersion(repoRoot) {
  const harness = resolve(repoRoot, "experiments/g0/harness");
  const source = [
    'import { chromium } from "@playwright/test";',
    "const browser = await chromium.launch({ headless: true });",
    "console.log(browser.version());",
    "await browser.close();",
  ].join("\n");
  return runCapture(
    process.execPath,
    ["--input-type=module", "--eval", source],
    harness,
  ).trim();
}
