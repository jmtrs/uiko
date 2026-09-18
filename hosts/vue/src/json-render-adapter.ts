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

export function toJsonRenderSpec(route: UiRoute): JsonRenderSpec {
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
          },
          children: [],
        };
        break;
      case "Table":
        elements[component.id] = {
          type: "Table",
          props: {
            uikoId: component.id,
            binding: component.binding,
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
