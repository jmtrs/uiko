import { Renderer, StateProvider } from "@json-render/vue";
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
  state: Readonly<Record<string, unknown>>,
): Promise<unknown> {
  const input: Record<string, unknown> = {};

  for (const binding of query.input) {
    let value: unknown;
    const routeParam = binding.expression.match(/^route\.(.+)$/)?.[1];
    const stateKey = binding.expression.match(/^state\.(.+)$/)?.[1];

    if (routeParam !== undefined) {
      value = params[routeParam];
    } else if (stateKey !== undefined) {
      value = state[stateKey];
    } else {
      throw new Error(
        `Unsupported compiled input ${binding.expression}`,
      );
    }

    if (value === null || value === undefined) {
      if (binding.required) {
        throw new Error(
          `Missing required compiled input ${binding.expression}`,
        );
      }
      continue;
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
  const localState = ref<Record<string, unknown>>(
    current.value === null ? {} : { ...current.value.route.state },
  );
  const queryData = ref<Record<string, unknown>>({});
  const loading = ref(current.value !== null);
  const queryError = ref<string | null>(null);
  let generation = 0;

  async function refresh(match: RouteMatch | null): Promise<void> {
    const requestGeneration = ++generation;
    queryError.value = null;

    if (match === null) {
      queryData.value = {};
      loading.value = false;
      return;
    }

    loading.value = true;
    try {
      const entries = await Promise.all(
        match.route.queries.map(async (query) => [
          query.alias,
          await invokeQuery(query, match.params, localState.value),
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

  function resetRoute(match: RouteMatch | null): void {
    current.value = match;
    localState.value = match === null ? {} : { ...match.route.state };
    void refresh(match);
  }

  window.addEventListener("popstate", () => {
    resetRoute(matchRoute(manifest.routes, window.location.pathname));
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
            h(
              StateProvider,
              {
                key: `${match.route.id}:${window.location.pathname}`,
                initialState: match.route.state,
                onStateChange: (
                  changes: Array<{ path: string; value: unknown }>,
                ) => {
                  let changed = false;
                  const next = { ...localState.value };
                  for (const change of changes) {
                    const key = change.path.replace(/^\//, "");
                    if (key.length > 0) {
                      next[key] = change.value;
                      changed = true;
                    }
                  }
                  if (changed) {
                    localState.value = next;
                    void refresh(match);
                  }
                },
              },
              {
                default: () =>
                  h(Renderer, {
                    spec: toJsonRenderSpec(match.route, queryData.value),
                    registry,
                  }),
              },
            ),
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
