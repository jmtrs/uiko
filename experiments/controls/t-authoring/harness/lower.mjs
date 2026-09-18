import { cp, mkdir, rm, writeFile } from "node:fs/promises";
import { dirname, relative, resolve, sep } from "node:path";
import { pathToFileURL } from "node:url";

const controlRoot = resolve(import.meta.dirname, "..");
const repoRoot = resolve(controlRoot, "../../..");
const generatedRoot = resolve(controlRoot, "generated");
const projectRoot = resolve(generatedRoot, "project");
const emittedApp = resolve(generatedRoot, "js/src/app.js");

await rm(projectRoot, { recursive: true, force: true });
await mkdir(projectRoot, { recursive: true });

const moduleUrl = pathToFileURL(emittedApp);
moduleUrl.searchParams.set("run", String(Date.now()));
const imported = await import(moduleUrl.href);
const project = imported.default;

if (!isProject(project)) {
  throw new Error("authored TypeScript did not export a valid ProjectSpec shape");
}

const integrationSource = resolve(repoRoot, "fixtures/support-console/integrations");
await cp(integrationSource, resolve(projectRoot, "integrations"), { recursive: true });

const moduleRefs = [];
for (const moduleSpec of project.modules) {
  const modulePath = confined(moduleSpec.file);
  moduleRefs.push(relativeRef(projectRoot, modulePath));

  const pageRefs = [];
  for (const pageSpec of moduleSpec.pages) {
    const pagePath = confined(pageSpec.file);
    pageRefs.push(relativeRef(dirname(modulePath), pagePath));

    const { file: _file, ...pageSource } = pageSpec;
    await writeJson(pagePath, {
      id: pageSource.id,
      route: pageSource.route,
      ...(pageSource.state === undefined ? {} : { state: pageSource.state }),
      ...(pageSource.queries === undefined ? {} : { queries: pageSource.queries }),
      components: pageSource.components ?? [],
    });
  }

  await writeJson(modulePath, {
    id: moduleSpec.id,
    pages: pageRefs,
  });
}

await writeJson(resolve(projectRoot, "uiko.jsonc"), {
  name: project.name,
  specVersion: project.specVersion,
  modules: moduleRefs,
  integrations: ["./integrations/crm.jsonc"],
});

function isProject(value) {
  return (
    typeof value === "object" &&
    value !== null &&
    typeof value.name === "string" &&
    value.specVersion === 1 &&
    Array.isArray(value.modules)
  );
}

function confined(path) {
  if (typeof path !== "string" || path.length === 0) {
    throw new Error("facade file paths must be non-empty strings");
  }
  const target = resolve(projectRoot, path);
  const prefix = projectRoot.endsWith(sep) ? projectRoot : `${projectRoot}${sep}`;
  if (target !== projectRoot && !target.startsWith(prefix)) {
    throw new Error(`facade file path escapes generated project: ${path}`);
  }
  return target;
}

function relativeRef(from, to) {
  const value = relative(from, to).split(sep).join("/");
  return value.startsWith(".") ? value : `./${value}`;
}

async function writeJson(path, value) {
  await mkdir(dirname(path), { recursive: true });
  await writeFile(path, `${JSON.stringify(value, null, 2)}\n`, "utf8");
}
