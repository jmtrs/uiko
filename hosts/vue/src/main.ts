import { JSONUIProvider, Renderer } from "@json-render/vue";
import { createApp, defineComponent, h, ref } from "vue";

import { registry } from "./catalog";
import { toJsonRenderSpec } from "./json-render-adapter";
import {
  uiManifestSchema,
  type UiManifest,
  type UiQuery,
  type UiStateValue,
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
  state: Readonly<Record<string, UiStateValue>>,
): Promise<unknown> {
  const input: Record<string, unknown> = {};

  for (const binding of query.input) {
    if (binding.expression.startsWith("route.")) {
      const routeParam = binding.expression.slice("route.".length);
      const value = params[routeParam];
      if (value === undefined) {
        throw new Error(`Missing route parameter ${routeParam}`);
      }
      input[binding.name] = value;
      continue;
    }

    if (binding.expression.startsWith("state.")) {
      const stateId = binding.expression.slice("state.".length);
      if (!(stateId in state)) {
        throw new Error(`Missing page state ${stateId}`);
      }
      const value = state[stateId];
      if (value !== null && value !== undefined) {
        input[binding.name] = value;
      }
      continue;
    }

    throw new Error(
      `Unsupported compiled input ${binding.expression}; expected route.* or state.*`,
    );
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
  const pageState = ref<Record<string, UiStateValue>>(
    initialState(current.value),
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
    const stateSnapshot = { ...pageState.value };
    try {
      const entries = await Promise.all(
        match.route.queries.map(
          async (query) =>
            [
              query.alias,
              await invokeQuery(query, match.params, stateSnapshot),
            ] as const,
        ),
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
    pageState.value = initialState(current.value);
    void refresh(current.value);
  });

  window.addEventListener("uiko-state-change", (event) => {
    const change = stateChange(event);
    if (change === null || !(change.state in pageState.value)) {
      return;
    }
    pageState.value = {
      ...pageState.value,
      [change.state]: change.value,
    };
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
            h(
              JSONUIProvider,
              { registry, initialState: {} },
              {
                default: () =>
                  h(Renderer, {
                    spec: toJsonRenderSpec(
                      match.route,
                      queryData.value,
                      pageState.value,
                    ),
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

function initialState(match: RouteMatch | null): Record<string, UiStateValue> {
  if (match === null) {
    return {};
  }
  return Object.fromEntries(
    match.route.state.map((state) => [state.id, state.initial]),
  );
}

function stateChange(
  event: Event,
): { state: string; value: UiStateValue } | null {
  if (!(event instanceof CustomEvent)) {
    return null;
  }
  const detail: unknown = event.detail;
  if (
    typeof detail !== "object" ||
    detail === null ||
    !("state" in detail) ||
    !("value" in detail)
  ) {
    return null;
  }

  const state = (detail as Record<string, unknown>).state;
  const value = (detail as Record<string, unknown>).value;
  if (
    typeof state !== "string" ||
    !(
      value === null ||
      typeof value === "string" ||
      (typeof value === "number" && Number.isInteger(value))
    )
  ) {
    return null;
  }
  return { state, value };
}

void bootstrap().catch((error: unknown) => {
  const message = error instanceof Error ? error.message : String(error);
  const root = document.querySelector<HTMLDivElement>("#app");
  if (root !== null) {
    root.textContent = message;
    root.dataset.uikoHost = "error";
  }
});
