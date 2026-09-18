import { z } from "zod";

export const uiStateValueSchema = z.union([
  z.string(),
  z.number().int(),
  z.null(),
]);

const stateSchema = z.object({
  id: z.string().min(1),
  initial: uiStateValueSchema,
});

const textComponentSchema = z.object({
  id: z.string().min(1),
  kind: z.literal("Text"),
  value: z.string(),
});

const fieldComponentSchema = z.object({
  id: z.string().min(1),
  kind: z.literal("Field"),
  label: z.string(),
  binding: z.string().min(1),
  fallback: z.string().optional(),
});

const tableComponentSchema = z.object({
  id: z.string().min(1),
  kind: z.literal("Table"),
  binding: z.string().min(1),
});

const selectComponentSchema = z.object({
  id: z.string().min(1),
  kind: z.literal("Select"),
  label: z.string(),
  state: z.string().min(1),
  options: z.array(
    z.object({
      label: z.string(),
      value: uiStateValueSchema,
    }),
  ),
});

const paginationComponentSchema = z.object({
  id: z.string().min(1),
  kind: z.literal("Pagination"),
  state: z.string().min(1),
  page: z.string().min(1),
  pageSize: z.string().min(1),
  total: z.string().min(1),
});

const queryInputSchema = z.object({
  name: z.string().min(1),
  expression: z.string().min(1),
});

const querySchema = z.object({
  id: z.string().min(1),
  alias: z.string().min(1),
  input: z.array(queryInputSchema),
});

export const uiComponentSchema = z.discriminatedUnion("kind", [
  textComponentSchema,
  fieldComponentSchema,
  tableComponentSchema,
  selectComponentSchema,
  paginationComponentSchema,
]);

export const uiRouteSchema = z.object({
  id: z.string().min(1),
  path: z.string().startsWith("/"),
  state: z.array(stateSchema),
  queries: z.array(querySchema),
  components: z.array(uiComponentSchema),
});

export const uiManifestSchema = z.object({
  specVersion: z.literal(1),
  app: z.string().min(1),
  routes: z.array(uiRouteSchema),
});

export type UiStateValue = z.infer<typeof uiStateValueSchema>;
export type UiComponent = z.infer<typeof uiComponentSchema>;
export type UiQuery = z.infer<typeof querySchema>;
export type UiRoute = z.infer<typeof uiRouteSchema>;
export type UiManifest = z.infer<typeof uiManifestSchema>;
