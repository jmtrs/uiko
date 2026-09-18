import { spawnSync } from "node:child_process";
import { copyFileSync, existsSync } from "node:fs";
import { resolve } from "node:path";

export function prepareExecutionBase({
  repoRoot,
  arm,
  taskId,
  prerequisiteManifest,
  frozenSetupRoot = null,
}) {
  requireClean(repoRoot);

  const prerequisite = runCapture(
    "bash",
    [
      "experiments/g0/materialize-prerequisite-base.sh",
      arm,
      taskId,
      ".",
    ],
    repoRoot,
  ).trim();

  setupHarness(repoRoot, frozenSetupRoot);

  switch (arm) {
    case "B_FULL":
      npmInstall(repoRoot, "baselines/b-full", frozenSetupRoot);
      runChecked("npm", ["run", "generate:api"], resolve(repoRoot, "baselines/b-full"));
      break;
    case "C_UIKO":
      npmInstall(repoRoot, "hosts/vue", frozenSetupRoot);
      runChecked(
        "cargo",
        ["build", "--quiet", "-p", "uiko-cli", "-p", "uiko-g0-gateway"],
        repoRoot,
      );
      break;
    case "R_RENDER_ONLY":
      npmInstall(repoRoot, "experiments/controls/r-render-only", frozenSetupRoot);
      runChecked(
        "npm",
        ["run", "generate:api"],
        resolve(repoRoot, "experiments/controls/r-render-only"),
      );
      break;
    case "T_AUTHORING":
      npmInstall(repoRoot, "hosts/vue", frozenSetupRoot);
      runChecked(
        process.execPath,
        ["experiments/g0/harness/prepare-t-authoring.mjs", "--root", repoRoot],
        repoRoot,
      );
      runChecked(
        "npm",
        ["exec", "--", "tsc", "-p", "t-authoring-tsconfig.json"],
        resolve(repoRoot, "experiments/g0/harness"),
      );
      runChecked(
        process.execPath,
        ["experiments/g0/harness/lower-t-authoring.mjs", "--root", repoRoot],
        repoRoot,
      );
      runChecked(
        "cargo",
        ["build", "--quiet", "-p", "uiko-cli", "-p", "uiko-g0-gateway"],
        repoRoot,
      );
      break;
    default:
      throw new Error(`unknown G0 arm ${arm}`);
  }

  runChecked("cargo", ["build", "--quiet", "-p", "uiko-g0-runner"], repoRoot);
  stageExecutionInputs(repoRoot, arm);

  const staged = runCapture("git", ["diff", "--cached", "--name-only"], repoRoot)
    .split(/\r?\n/)
    .filter(Boolean);

  if (staged.length === 0) {
    return {
      prerequisiteRevision: prerequisite,
      baseRevision: prerequisite,
      stagedSetupPaths: [],
    };
  }

  const tree = runCapture("git", ["write-tree"], repoRoot).trim();
  const timestamp = prerequisiteManifest.syntheticCommit.timestamp;
  const env = {
    ...process.env,
    GIT_AUTHOR_NAME: prerequisiteManifest.syntheticCommit.authorName,
    GIT_AUTHOR_EMAIL: prerequisiteManifest.syntheticCommit.authorEmail,
    GIT_AUTHOR_DATE: timestamp,
    GIT_COMMITTER_NAME: prerequisiteManifest.syntheticCommit.authorName,
    GIT_COMMITTER_EMAIL: prerequisiteManifest.syntheticCommit.authorEmail,
    GIT_COMMITTER_DATE: timestamp,
  };
  const message = `g0 execution base ${arm} for ${taskId}`;
  const commit = spawnSync(
    "git",
    ["commit-tree", tree, "-p", prerequisite],
    {
      cwd: repoRoot,
      env,
      input: `${message}\n`,
      encoding: "utf8",
    },
  );
  if (commit.status !== 0) {
    throw new Error(`git commit-tree failed: ${commit.stderr}`);
  }
  const baseRevision = commit.stdout.trim();
  runChecked("git", ["reset", "--hard", "--quiet", baseRevision], repoRoot);
  requireClean(repoRoot);

  return {
    prerequisiteRevision: prerequisite,
    baseRevision,
    stagedSetupPaths: staged,
  };
}

export function cleanupHarnessArtifacts(repoRoot, arm) {
  const transient = [];

  if (arm === "C_UIKO" || arm === "T_AUTHORING") {
    transient.push("hosts/vue/public/app.uiko-manifest.json");
  }

  for (const path of transient) {
    runChecked("git", ["clean", "-f", "--", path], repoRoot, {
      allowFailure: true,
    });
  }
}

function setupHarness(repoRoot, frozenSetupRoot) {
  npmInstall(repoRoot, "experiments/g0/harness", frozenSetupRoot);
  runChecked(
    "npx",
    ["playwright", "install", "chromium"],
    resolve(repoRoot, "experiments/g0/harness"),
  );
}

function npmInstall(repoRoot, path, frozenSetupRoot) {
  const cwd = resolve(repoRoot, path);

  if (frozenSetupRoot !== null) {
    const frozenLock = resolve(frozenSetupRoot, path, "package-lock.json");
    if (!existsSync(frozenLock)) {
      throw new Error(`missing frozen npm lock snapshot: ${frozenLock}`);
    }
    copyFileSync(frozenLock, resolve(cwd, "package-lock.json"));
    runChecked(
      "npm",
      ["ci", "--ignore-scripts", "--no-audit", "--no-fund"],
      cwd,
    );
    return;
  }

  runChecked(
    "npm",
    ["install", "--ignore-scripts", "--no-audit", "--no-fund"],
    cwd,
  );
}

function stageExecutionInputs(repoRoot, arm) {
  const paths = ["experiments/g0/harness/package-lock.json"];

  if (arm === "B_FULL") {
    paths.push(
      "baselines/b-full/package-lock.json",
      "baselines/b-full/src/api/generated/schema.d.ts",
    );
  } else if (arm === "C_UIKO") {
    paths.push("hosts/vue/package-lock.json");
  } else if (arm === "R_RENDER_ONLY") {
    paths.push(
      "experiments/controls/r-render-only/package-lock.json",
      "experiments/controls/r-render-only/src/api/generated/schema.d.ts",
    );
  } else if (arm === "T_AUTHORING") {
    paths.push(
      "hosts/vue/package-lock.json",
      "experiments/controls/t-authoring/generated",
    );
  }

  const existing = paths.filter((path) => existsSync(resolve(repoRoot, path)));
  if (existing.length > 0) {
    runChecked("git", ["add", "-f", "--", ...existing], repoRoot);
  }
}

function gitPathIsTracked(repoRoot, path) {
  const result = spawnSync("git", ["ls-files", "--error-unmatch", "--", path], {
    cwd: repoRoot,
    stdio: "ignore",
  });
  if (result.status === 0) {
    return true;
  }
  if (result.status === 1) {
    return false;
  }
  throw new Error(`cannot determine whether ${path} is tracked`);
}

function requireClean(repoRoot) {
  const status = runCapture(
    "git",
    ["status", "--porcelain", "--untracked-files=all"],
    repoRoot,
  );
  if (status.trim().length > 0) {
    throw new Error(`G0 worktree is not clean:\n${status}`);
  }
}

function runCapture(command, args, cwd) {
  const result = spawnSync(command, args, { cwd, encoding: "utf8" });
  if (result.status !== 0) {
    throw new Error(
      `${command} ${args.join(" ")} failed: ${result.stderr || result.stdout}`,
    );
  }
  return result.stdout;
}

function runChecked(command, args, cwd, options = {}) {
  const result = spawnSync(command, args, {
    cwd,
    stdio: options.allowFailure ? "ignore" : "inherit",
  });
  if (result.status !== 0 && !options.allowFailure) {
    throw new Error(
      `${command} ${args.join(" ")} failed with exit code ${result.status}`,
    );
  }
}
