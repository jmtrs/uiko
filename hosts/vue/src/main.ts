import { Renderer } from "@json-render/vue";
import { createApp, defineComponent, h, ref } from "vue";

import { registry } from "./catalog";
import { toJsonRenderSpec } from "./json-render-adapter";
import {
  uiManifestSchema,
  type UiManifest,
  type UiQuery,
} from "./manifest";
import { matchRoute, type RouteMatch } from "./router";

const MANIFEST_URL = "/app.uiko-manifest.json";
const QUERY_URL = "/__uiko/query";

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

async function invokeQuery(
  query: UiQuery,
  params: Readonly<Record<string, string>>,
): Promise<unknown> {
  const input: Record<string, string> = {};

  for (const binding of query.input) {
    const routeParam = binding.expression.replace(/^route\./, "");
    if (routeParam === binding.expression || !(routeParam in params)) {
      throw new Error(
        `Unsupported or unresolved compiled input ${binding.expression}`,
      );
    }
    const value = params[routeParam];
    if (value === undefined) {
      throw new Error(`Missing route parameter ${routeParam}`);
    }
    input[binding.name] = value;
  }

  const response = await fetch(QUERY_URL, {
    method: "POST",
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      operationId: query.id,
      input,
    }),
  });

  const body: unknown = await response.json();
  if (!response.ok) {
    throw new Error(
      `Logical query ${query.id} failed with HTTP ${response.status}`,
    );
  }
  return body;
}

async function bootstrap(): Promise<void> {
  const manifest = await loadManifest();
  const current = ref<RouteMatch | null>(
    matchRoute(manifest.routes, window.location.pathname),
  );
  const queryData = ref<Record<string, unknown>>({});
  const loading = ref(current.value !== null);
  const queryError = ref<string | null>(null);
  let generation = 0;

  async function refresh(match: RouteMatch | null): Promise<void> {
    const requestGeneration = ++generation;
    queryData.value = {};
    queryError.value = null;

    if (match === null) {
      loading.value = false;
      return;
    }

    loading.value = true;
    try {
      const entries = await Promise.all(
        match.route.queries.map(async (query) => [
          query.alias,
          await invokeQuery(query, match.params),
        ] as const),
      );
      if (requestGeneration === generation) {
        queryData.value = Object.fromEntries(entries);
      }
    } catch (error: unknown) {
      if (requestGeneration === generation) {
        queryError.value = error instanceof Error ? error.message : String(error);
      }
    } finally {
      if (requestGeneration === generation) {
        loading.value = false;
      }
    }
  }

  window.addEventListener("popstate", () => {
    current.value = matchRoute(manifest.routes, window.location.pathname);
    void refresh(current.value);
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

        const status =
          loading.value
            ? h("p", { role: "status", "aria-live": "polite" }, "Loading…")
            : queryError.value === null
              ? null
              : h(
                  "p",
                  { role: "alert", "data-uiko-query-error": "" },
                  queryError.value,
                );

        return h(
          "main",
          {
            "data-uiko-host": "ready",
            "data-uiko-route": match.route.id,
          },
          [
            status,
            h(Renderer, {
              spec: toJsonRenderSpec(match.route, queryData.value),
              registry,
            }),
          ],
        );
      };
    },
  });

  createApp(App).mount("#app");
  void refresh(current.value);
}

void bootstrap().catch((error: unknown) => {
  const message = error instanceof Error ? error.message : String(error);
  const root = document.querySelector<HTMLDivElement>("#app");
  if (root !== null) {
    root.textContent = message;
    root.dataset.uikoHost = "error";
  }
});
