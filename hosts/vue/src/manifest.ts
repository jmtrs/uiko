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

export const uiComponentSchema = z.discriminatedUnion("kind", [
  textComponentSchema,
  fieldComponentSchema,
  tableComponentSchema,
]);

export const uiRouteSchema = z.object({
  id: z.string().min(1),
  path: z.string().startsWith("/"),
  components: z.array(uiComponentSchema),
});

export const uiManifestSchema = z.object({
  specVersion: z.literal(1),
  app: z.string().min(1),
  routes: z.array(uiRouteSchema),
});

export type UiComponent = z.infer<typeof uiComponentSchema>;
export type UiRoute = z.infer<typeof uiRouteSchema>;
export type UiManifest = z.infer<typeof uiManifestSchema>;
