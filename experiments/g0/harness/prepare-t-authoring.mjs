import { mkdir, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { parseArgs } from "node:util";

const { values } = parseArgs({
  options: {
    root: { type: "string", default: "../../../" },
  },
});

const repoRoot = resolve(values.root);
const output = resolve(
  repoRoot,
  "experiments/controls/t-authoring/generated/uiko-authoring.d.ts",
);

await mkdir(dirname(output), { recursive: true });
await writeFile(
  output,
  `export type StateValue = string | number | null;

export interface UikoApp {
  name: string;
  specVersion: 1;
  integrations: readonly ["crm"];
  modules: readonly UikoModule[];
}

export interface UikoModule {
  id: string;
  pages: readonly UikoPage[];
}

export interface UikoPage {
  id: string;
  route: \`/\${string}\`;
  state?: Readonly<Record<string, StateValue>>;
  queries?: Readonly<Record<string, UikoQuery>>;
  components: readonly UikoComponent[];
}

export interface UikoQuery {
  operation: \`crm.\${string}\`;
  input: Readonly<Record<string, string>>;
}

export type UikoComponent =
  | { id: string; type: "Text"; value: string }
  | {
      id: string;
      type: "Field";
      label: string;
      binding: string;
      fallback?: string;
    }
  | { id: string; type: "Table"; binding: string }
  | {
      id: string;
      type: "Select";
      label: string;
      state: string;
      options: readonly { label: string; value: StateValue }[];
    }
  | {
      id: string;
      type: "Pagination";
      state: string;
      page: string;
      pageSize: string;
      total: string;
    };
`,
  "utf8",
);
process.stdout.write(`${output}\n`);
