import type { Spec } from "@json-render/core";
import { Renderer } from "@json-render/react";

import { registry } from "./catalog";

export interface JsonUiProps {
  spec: Spec;
  loading?: boolean;
}

export function JsonUi({ spec, loading = false }: JsonUiProps) {
  return <Renderer spec={spec} registry={registry} loading={loading} />;
}
