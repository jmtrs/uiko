import { mkdir } from "node:fs/promises";
import { join } from "node:path";

import { runChecked, spawnManaged, stopManaged, waitForHttp, waitForTcp } from "./processes.mjs";

const FIXTURE_PORT = 4317;
const B_FULL_PORT = 4318;
const C_UIKO_PORT = 4319;
const R_RENDER_ONLY_PORT = 4320;
const T_AUTHORING_PORT = 4321;
const GATEWAY_PORT = 3001;

export async function launchArm(repoRoot, arm) {
  const children = [];
  const fixtureUrl = `http://127.0.0.1:${FIXTURE_PORT}`;

  try {
    const fixture = spawnManaged(
      process.execPath,
      ["experiments/g0/harness/fixture-server.mjs", "--port", String(FIXTURE_PORT)],
      { cwd: repoRoot, env: { ...process.env, G0_FIXTURE_DELAY_MS: "300" } },
    );
    children.push(fixture);
    await waitForHttp(`${fixtureUrl}/__harness/health`);

    if (arm === "B_FULL") {
      const child = spawnManaged(
        "npm",
        ["run", "dev", "--", "--host", "127.0.0.1", "--port", String(B_FULL_PORT), "--strictPort"],
        {
          cwd: join(repoRoot, "baselines/b-full"),
          env: { ...process.env, VITE_API_BASE_URL: fixtureUrl },
        },
      );
      children.push(child);
      const baseUrl = `http://127.0.0.1:${B_FULL_PORT}`;
      await waitForHttp(baseUrl);
      return session(baseUrl, fixtureUrl, children);
    }

    if (arm === "R_RENDER_ONLY") {
      const child = spawnManaged(
        "npm",
        [
          "run",
          "dev",
          "--",
          "--host",
          "127.0.0.1",
          "--port",
          String(R_RENDER_ONLY_PORT),
          "--strictPort",
        ],
        {
          cwd: join(repoRoot, "experiments/controls/r-render-only"),
          env: { ...process.env, VITE_API_BASE_URL: fixtureUrl },
        },
      );
      children.push(child);
      const baseUrl = `http://127.0.0.1:${R_RENDER_ONLY_PORT}`;
      await waitForHttp(baseUrl);
      return session(baseUrl, fixtureUrl, children);
    }

    if (arm === "T_AUTHORING") {
      runChecked(
        process.execPath,
        [
          "experiments/g0/harness/prepare-t-authoring.mjs",
          "--root",
          repoRoot,
        ],
        { cwd: repoRoot },
      );
      runChecked(
        "npm",
        [
          "exec",
          "--",
          "tsc",
          "-p",
          "t-authoring-tsconfig.json",
        ],
        { cwd: join(repoRoot, "experiments/g0/harness") },
      );
      runChecked(
        process.execPath,
        [
          "experiments/g0/harness/lower-t-authoring.mjs",
          "--root",
          repoRoot,
        ],
        { cwd: repoRoot },
      );

      const project = "experiments/controls/t-authoring/generated/project";
      const publicDir = join(repoRoot, "hosts/vue/public");
      await mkdir(publicDir, { recursive: true });
      runChecked(
        "cargo",
        [
          "run",
          "--quiet",
          "-p",
          "uiko-cli",
          "--",
          "build",
          project,
          "--ui-manifest",
          "hosts/vue/public/app.uiko-manifest.json",
        ],
        { cwd: repoRoot },
      );

      const gateway = spawnManaged(
        "cargo",
        [
          "run",
          "--quiet",
          "-p",
          "uiko-g0-gateway",
          "--",
          project,
          "--listen",
          `127.0.0.1:${GATEWAY_PORT}`,
          "--integration",
          `crm=${fixtureUrl}`,
        ],
        { cwd: repoRoot },
      );
      children.push(gateway);
      await waitForTcp(GATEWAY_PORT);

      const host = spawnManaged(
        "npm",
        [
          "run",
          "dev",
          "--",
          "--host",
          "127.0.0.1",
          "--port",
          String(T_AUTHORING_PORT),
          "--strictPort",
        ],
        { cwd: join(repoRoot, "hosts/vue"), env: process.env },
      );
      children.push(host);
      const baseUrl = `http://127.0.0.1:${T_AUTHORING_PORT}`;
      await waitForHttp(baseUrl);
      return session(baseUrl, fixtureUrl, children);
    }

    if (arm === "C_UIKO") {
      const publicDir = join(repoRoot, "hosts/vue/public");
      await mkdir(publicDir, { recursive: true });
      runChecked(
        "cargo",
        [
          "run",
          "--quiet",
          "-p",
          "uiko-cli",
          "--",
          "build",
          "fixtures/support-console",
          "--ui-manifest",
          "hosts/vue/public/app.uiko-manifest.json",
        ],
        { cwd: repoRoot },
      );

      const gateway = spawnManaged(
        "cargo",
        [
          "run",
          "--quiet",
          "-p",
          "uiko-g0-gateway",
          "--",
          "fixtures/support-console",
          "--listen",
          `127.0.0.1:${GATEWAY_PORT}`,
          "--integration",
          `crm=${fixtureUrl}`,
        ],
        { cwd: repoRoot },
      );
      children.push(gateway);
      await waitForTcp(GATEWAY_PORT);

      const host = spawnManaged(
        "npm",
        ["run", "dev", "--", "--host", "127.0.0.1", "--port", String(C_UIKO_PORT), "--strictPort"],
        { cwd: join(repoRoot, "hosts/vue"), env: process.env },
      );
      children.push(host);
      const baseUrl = `http://127.0.0.1:${C_UIKO_PORT}`;
      await waitForHttp(baseUrl);
      return session(baseUrl, fixtureUrl, children);
    }

    throw new Error(`Unsupported G0 arm ${arm}`);
  } catch (error) {
    await stopAll(children);
    throw error;
  }
}

function session(baseUrl, fixtureUrl, children) {
  return {
    baseUrl,
    fixtureUrl,
    async stop() {
      await stopAll(children);
    },
  };
}

async function stopAll(children) {
  for (const child of [...children].reverse()) {
    await stopManaged(child);
  }
}
