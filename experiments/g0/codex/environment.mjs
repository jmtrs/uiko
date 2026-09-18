import { createHash } from "node:crypto";
import { existsSync } from "node:fs";
import { readFile, realpath } from "node:fs/promises";
import { createRequire } from "node:module";
import {
  basename,
  dirname,
  isAbsolute,
  join,
  resolve,
} from "node:path";
import { spawnSync } from "node:child_process";

const CODEX_TARGET_BY_RUNTIME = {
  "linux:x64": {
    targetTriple: "x86_64-unknown-linux-musl",
    packageName: "@openai/codex-linux-x64",
  },
  "linux:arm64": {
    targetTriple: "aarch64-unknown-linux-musl",
    packageName: "@openai/codex-linux-arm64",
  },
  "darwin:x64": {
    targetTriple: "x86_64-apple-darwin",
    packageName: "@openai/codex-darwin-x64",
  },
  "darwin:arm64": {
    targetTriple: "aarch64-apple-darwin",
    packageName: "@openai/codex-darwin-arm64",
  },
  "win32:x64": {
    targetTriple: "x86_64-pc-windows-msvc",
    packageName: "@openai/codex-win32-x64",
  },
  "win32:arm64": {
    targetTriple: "aarch64-pc-windows-msvc",
    packageName: "@openai/codex-win32-arm64",
  },
};

export async function inspectEnvironment(repoRoot, codexBin = "codex") {
  return {
    os: process.platform,
    arch: process.arch,
    node: process.version,
    npm: runCapture("npm", ["--version"], repoRoot).trim(),
    rustc: runCapture("rustc", ["--version"], repoRoot).trim(),
    browser: browserVersion(repoRoot),
    codex: await inspectCodexIdentity(repoRoot, codexBin),
  };
}

export async function inspectCodexIdentity(repoRoot, codexBin = "codex") {
  const versionOutput = runCapture(codexBin, ["--version"], repoRoot).trim();
  const launcherExecutable = await commandExecutable(repoRoot, codexBin);
  const executable = await resolveCodexNativeExecutable(launcherExecutable);

  return {
    versionOutput,
    launcherExecutable,
    launcherSha256: await sha256File(launcherExecutable),
    executable,
    sha256: await sha256File(executable),
  };
}

export async function resolveCodexNativeExecutable(
  launcherExecutable,
  platform = process.platform,
  arch = process.arch,
) {
  const launcher = await realpath(launcherExecutable);
  if (basename(launcher) !== "codex.js") {
    return launcher;
  }

  const target = CODEX_TARGET_BY_RUNTIME[`${platform}:${arch}`];
  if (target === undefined) {
    throw new Error(`unsupported Codex runtime: ${platform} (${arch})`);
  }

  const codexPackageRoot = dirname(dirname(launcher));
  const require = createRequire(launcher);
  let vendorRoot;
  try {
    const packageJsonPath = require.resolve(
      `${target.packageName}/package.json`,
    );
    vendorRoot = join(dirname(packageJsonPath), "vendor");
  } catch {
    vendorRoot = join(codexPackageRoot, "vendor");
  }

  const executable = join(
    vendorRoot,
    target.targetTriple,
    "bin",
    platform === "win32" ? "codex.exe" : "codex",
  );
  if (!existsSync(executable)) {
    throw new Error(
      `Codex native executable not found for ${platform}/${arch}: ${executable}`,
    );
  }
  return realpath(executable);
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
      `Codex native executable mismatch: expected ${lock.agent.codexExecutableSha256}, got ${actual.codex.sha256}`,
    );
  }
  if (lock.agent.codexLauncherSha256 !== actual.codex.launcherSha256) {
    throw new Error(
      `Codex launcher mismatch: expected ${lock.agent.codexLauncherSha256}, got ${actual.codex.launcherSha256}`,
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

async function commandExecutable(repoRoot, command) {
  const containsPathSeparator = command.includes("/") || command.includes("\\");
  const candidate =
    isAbsolute(command) || containsPathSeparator
      ? resolve(repoRoot, command)
      : runCapture("which", [command], repoRoot).trim();
  return realpath(candidate);
}

async function sha256File(path) {
  const bytes = await readFile(path);
  return createHash("sha256").update(bytes).digest("hex");
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
