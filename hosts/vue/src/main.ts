import { Renderer } from "@json-render/vue";
import { createApp, defineComponent, h, ref } from "vue";

import { registry } from "./catalog";
import { toJsonRenderSpec } from "./json-render-adapter";
import { uiManifestSchema, type UiManifest } from "./manifest";
import { matchRoute, type RouteMatch } from "./router";

const MANIFEST_URL = "/app.uiko-manifest.json";

async function loadManifest(): Promise<UiManifest> {
  const response = await fetch(MANIFEST_URL, {
    headers: {
      Accept: "application/json",
    },
  });

  if (!response.ok) {
    throw new Error(
      `Unable to load ${MANIFEST_URL}: HTTP ${response.status}. Generate it with the uiko CLI first.`,
    );
  }

  return uiManifestSchema.parse(await response.json());
}

async function bootstrap(): Promise<void> {
  const manifest = await loadManifest();
  const current = ref<RouteMatch | null>(
    matchRoute(manifest.routes, window.location.pathname),
  );

  window.addEventListener("popstate", () => {
    current.value = matchRoute(manifest.routes, window.location.pathname);
  });

  const App = defineComponent({
    name: "UikoHost",
    setup() {
      return () => {
        const match = current.value;
        if (match === null) {
          return h(
            "main",
            {
              "data-uiko-host": "not-found",
            },
            `No uiko route matches ${window.location.pathname}`,
          );
        }

        return h(
          "main",
          {
            "data-uiko-host": "ready",
            "data-uiko-route": match.route.id,
          },
          [
            h(Renderer, {
              spec: toJsonRenderSpec(match.route),
              registry,
            }),
          ],
        );
      };
    },
  });

  createApp(App).mount("#app");
}

void bootstrap().catch((error: unknown) => {
  const message = error instanceof Error ? error.message : String(error);
  const root = document.querySelector<HTMLDivElement>("#app");
  if (root !== null) {
    root.textContent = message;
    root.dataset.uikoHost = "error";
  }
});
