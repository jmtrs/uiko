import { z } from "zod";

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
});

const tableComponentSchema = z.object({
  id: z.string().min(1),
  kind: z.literal("Table"),
  binding: z.string().min(1),
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
]);

export const uiRouteSchema = z.object({
  id: z.string().min(1),
  path: z.string().startsWith("/"),
  queries: z.array(querySchema),
  components: z.array(uiComponentSchema),
});

export const uiManifestSchema = z.object({
  specVersion: z.literal(1),
  app: z.string().min(1),
  routes: z.array(uiRouteSchema),
});

export type UiComponent = z.infer<typeof uiComponentSchema>;
export type UiQuery = z.infer<typeof querySchema>;
export type UiRoute = z.infer<typeof uiRouteSchema>;
export type UiManifest = z.infer<typeof uiManifestSchema>;
