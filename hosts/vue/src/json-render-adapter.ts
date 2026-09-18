import type { UiRoute } from "./manifest";

export interface JsonRenderElement {
  type: "Stack" | "Text" | "Field" | "Table";
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
      case "Field":
        elements[component.id] = {
          type: "Field",
          props: {
            uikoId: component.id,
            label: component.label,
            binding: component.binding,
            value: resolveBinding(queryData, component.binding),
          },
          children: [],
        };
        break;
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
    }
  }

  return {
    root,
    elements,
    state: {},
  };
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
