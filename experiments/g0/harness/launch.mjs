import { mkdir } from "node:fs/promises";
import { join } from "node:path";

import { runChecked, spawnManaged, stopManaged, waitForHttp } from "./processes.mjs";

const FIXTURE_PORT = 4317;
const B_FULL_PORT = 4318;
const C_UIKO_PORT = 4319;
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

    throw new Error(`Unsupported primary arm ${arm}`);
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
