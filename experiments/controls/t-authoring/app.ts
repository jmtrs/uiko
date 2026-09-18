import type { UikoApp } from "./generated/uiko-authoring.js";

const app = {
  name: "support-console",
  specVersion: 1,
  integrations: ["crm"],
  modules: [],
} satisfies UikoApp;

export default app;
