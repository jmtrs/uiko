import { copyFile, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { parseArgs } from "node:util";

import ts from "typescript";

const { values } = parseArgs({
  options: {
    root: { type: "string", default: "../../../" },
    source: {
      type: "string",
      default: "experiments/controls/t-authoring/app.ts",
    },
    out: {
      type: "string",
      default: "experiments/controls/t-authoring/generated/project",
    },
  },
});

const repoRoot = resolve(values.root);
const sourcePath = resolve(repoRoot, values.source);
const outputRoot = resolve(repoRoot, values.out);
const runtimeDir = resolve(repoRoot, "experiments/controls/t-authoring/generated/runtime");

await rm(outputRoot, { recursive: true, force: true });
await rm(runtimeDir, { recursive: true, force: true });
await mkdir(runtimeDir, { recursive: true });

const sourceText = await readFile(sourcePath, "utf8");
const transpiled = ts.transpileModule(sourceText, {
  compilerOptions: {
    target: ts.ScriptTarget.ES2022,
    module: ts.ModuleKind.ESNext,
    verbatimModuleSyntax: true,
  },
  fileName: sourcePath,
});

const runtimeModule = resolve(runtimeDir, "app.mjs");
await writeFile(runtimeModule, transpiled.outputText, "utf8");
const loaded = await import(`${pathToFileURL(runtimeModule).href}?uiko-g0`);
const app = loaded.default;

assertRecord(app, "default export");
if (!Array.isArray(app.modules)) {
  throw new Error("T_AUTHORING default export must contain a modules array");
}
if (!Array.isArray(app.integrations) || app.integrations.length !== 1 || app.integrations[0] !== "crm") {
  throw new Error('T_AUTHORING integrations must be exactly ["crm"] for the frozen G0 probe');
}

await mkdir(outputRoot, { recursive: true });
await mkdir(resolve(outputRoot, "integrations/openapi"), { recursive: true });
await copyFile(
  resolve(repoRoot, "experiments/fixtures/g0-support-api.openapi.json"),
  resolve(outputRoot, "integrations/openapi/crm.json"),
);
await writeJson(resolve(outputRoot, "integrations/crm.jsonc"), {
  id: "crm",
  adapter: "uiko.openapi-http",
  contract: "./openapi/crm.json",
});

const moduleRefs = [];
for (const [moduleIndex, module] of app.modules.entries()) {
  assertRecord(module, `modules[${moduleIndex}]`);
  if (!Array.isArray(module.pages)) {
    throw new Error(`modules[${moduleIndex}].pages must be an array`);
  }

  const moduleDirName = `module-${moduleIndex}`;
  const moduleDir = resolve(outputRoot, "features", moduleDirName);
  await mkdir(moduleDir, { recursive: true });
  moduleRefs.push(`./features/${moduleDirName}`);

  const pageRefs = [];
  for (const [pageIndex, page] of module.pages.entries()) {
    assertRecord(page, `modules[${moduleIndex}].pages[${pageIndex}]`);
    const pageName = `page-${pageIndex}.jsonc`;
    pageRefs.push(`./${pageName}`);
    await writeJson(resolve(moduleDir, pageName), page);
  }

  await writeJson(resolve(moduleDir, "module.jsonc"), {
    id: requiredString(module.id, `modules[${moduleIndex}].id`),
    pages: pageRefs,
  });
}

await writeJson(resolve(outputRoot, "uiko.jsonc"), {
  name: requiredString(app.name, "name"),
  specVersion: app.specVersion,
  modules: moduleRefs,
  integrations: ["./integrations/crm.jsonc"],
});

process.stdout.write(`${outputRoot}\n`);

async function writeJson(path, value) {
  await mkdir(dirname(path), { recursive: true });
  await writeFile(path, `${JSON.stringify(value, null, 2)}\n`, "utf8");
}

function assertRecord(value, label) {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error(`${label} must be an object`);
  }
}

function requiredString(value, label) {
  if (typeof value !== "string" || value.length === 0) {
    throw new Error(`${label} must be a non-empty string`);
  }
  return value;
}
