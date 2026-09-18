import { defineCatalog } from "@json-render/core";
import { defineRegistry } from "@json-render/react";
import { schema } from "@json-render/react/schema";
import { z } from "zod";

export const catalog = defineCatalog(schema, {
  components: {
    Stack: {
      props: z.object({
        gap: z.number().nonnegative().nullable(),
      }),
      description: "Generic vertical stack.",
    },
    Text: {
      props: z.object({
        value: z.string(),
      }),
      description: "Plain text.",
    },
    Field: {
      props: z.object({
        label: z.string(),
        value: z.union([z.string(), z.number(), z.boolean(), z.null()]),
      }),
      description: "Label/value pair.",
    },
    Table: {
      props: z.object({
        columns: z.array(z.string()),
        rows: z.array(z.record(z.string(), z.unknown())),
      }),
      description: "Generic tabular data.",
    },
  },
  actions: {},
});

export const { registry } = defineRegistry(catalog, {
  components: {
    Stack: ({ props, children }) => (
      <section style={{ display: "grid", gap: props.gap ?? 8 }}>{children}</section>
    ),
    Text: ({ props }) => <p>{props.value}</p>,
    Field: ({ props }) => (
      <div>
        <strong>{props.label}</strong>{" "}
        <span>{props.value === null ? "" : String(props.value)}</span>
      </div>
    ),
    Table: ({ props }) => (
      <table>
        <thead>
          <tr>
            {props.columns.map((column) => (
              <th key={column} scope="col">
                {column}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {props.rows.map((row, rowIndex) => (
            <tr key={String(row.id ?? rowIndex)}>
              {props.columns.map((column) => (
                <td key={column}>{formatCell(row[column])}</td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    ),
  },
});

function formatCell(value: unknown): string {
  if (value === null || value === undefined) {
    return "";
  }
  return typeof value === "object" ? JSON.stringify(value) : String(value);
}
