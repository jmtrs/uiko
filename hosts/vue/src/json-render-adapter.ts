import type { UiRoute, UiStateValue } from "./manifest";

export interface JsonRenderElement {
  type: "Stack" | "Text" | "Field" | "Table" | "Select" | "Pagination";
  props: Record<string, unknown>;
  children: string[];
}

export interface JsonRenderSpec {
  root: string;
  elements: Record<string, JsonRenderElement>;
  state: Record<string, unknown>;
}

export function toJsonRenderSpec(
  route: UiRoute,
  queryData: Readonly<Record<string, unknown>>,
  pageState: Readonly<Record<string, UiStateValue>>,
): JsonRenderSpec {
  const root = `${route.id}.root`;
  const elements: Record<string, JsonRenderElement> = {
    [root]: {
      type: "Stack",
      props: {
        uikoId: route.id,
      },
      children: route.components.map((component) => component.id),
    },
  };

  for (const component of route.components) {
    switch (component.kind) {
      case "Text":
        elements[component.id] = {
          type: "Text",
          props: {
            uikoId: component.id,
            value: component.value,
          },
          children: [],
        };
        break;
      case "Field": {
        const resolved = resolveBinding(queryData, component.binding);
        elements[component.id] = {
          type: "Field",
          props: {
            uikoId: component.id,
            label: component.label,
            binding: component.binding,
            value:
              (resolved === null || resolved === undefined) &&
              component.fallback !== undefined
                ? component.fallback
                : resolved,
          },
          children: [],
        };
        break;
      }
      case "Table": {
        const resolved = resolveBinding(queryData, component.binding);
        elements[component.id] = {
          type: "Table",
          props: {
            uikoId: component.id,
            binding: component.binding,
            rows: Array.isArray(resolved) ? resolved : [],
          },
          children: [],
        };
        break;
      }
      case "Select":
        elements[component.id] = {
          type: "Select",
          props: {
            uikoId: component.id,
            label: component.label,
            state: component.state,
            value: pageState[component.state] ?? null,
            options: component.options,
          },
          children: [],
        };
        break;
      case "Pagination":
        elements[component.id] = {
          type: "Pagination",
          props: {
            uikoId: component.id,
            state: component.state,
            value: pageState[component.state] ?? null,
            page: numericBinding(queryData, component.page),
            pageSize: numericBinding(queryData, component.pageSize),
            total: numericBinding(queryData, component.total),
          },
          children: [],
        };
        break;
    }
  }

  return {
    root,
    elements,
    state: {},
  };
}

function numericBinding(
  queryData: Readonly<Record<string, unknown>>,
  binding: string,
): number | null {
  const value = resolveBinding(queryData, binding);
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function resolveBinding(
  queryData: Readonly<Record<string, unknown>>,
  binding: string,
): unknown {
  const [alias, ...path] = binding.split(".");
  if (alias === undefined || alias.length === 0) {
    return undefined;
  }

  let value: unknown = queryData[alias];
  for (const segment of path) {
    if (
      typeof value !== "object" ||
      value === null ||
      Array.isArray(value) ||
      !(segment in value)
    ) {
      return undefined;
    }
    value = (value as Record<string, unknown>)[segment];
  }
  return value;
}
