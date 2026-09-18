import type { UiRoute } from "./manifest";

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
        const value = resolveBinding(queryData, component.binding);
        elements[component.id] = {
          type: "Field",
          props: {
            uikoId: component.id,
            label: component.label,
            binding: component.binding,
            value:
              value === null || value === undefined
                ? (component.fallback ?? "")
                : value,
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
            columns: component.columns,
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
            value: { $bindState: `/${component.state}` },
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
            pageValue: { $bindState: `/${component.pageState}` },
            page: resolveBinding(queryData, component.page),
            pageSize: resolveBinding(queryData, component.pageSize),
            total: resolveBinding(queryData, component.total),
          },
          children: [],
        };
        break;
    }
  }

  return {
    root,
    elements,
    state: route.state,
  };
}

export function resolveBinding(
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

export function resolveRowBinding(
  row: Readonly<Record<string, unknown>>,
  binding: string,
): unknown {
  let value: unknown = row;
  for (const segment of binding.split(".")) {
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
