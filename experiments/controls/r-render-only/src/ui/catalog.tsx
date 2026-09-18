import { defineCatalog } from "@json-render/core";
import { defineRegistry } from "@json-render/react";
import { schema } from "@json-render/react/schema";
import { z } from "zod";

export const catalog = defineCatalog(schema, {
  components: {
    Stack: {
      props: z.object({}),
      description: "Vertical container.",
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
        value: z.unknown(),
      }),
      description: "Label and scalar display value.",
    },
    Table: {
      props: z.object({
        columns: z.array(
          z.object({
            label: z.string(),
            key: z.string(),
          }),
        ),
        rows: z.array(z.record(z.string(), z.unknown())),
      }),
      description: "Tabular records with explicit column projection.",
    },
  },
  actions: {},
});

export const { registry } = defineRegistry(catalog, {
  components: {
    Stack: ({ children }) => <section>{children}</section>,
    Text: ({ props }) => <p>{props.value}</p>,
    Field: ({ props }) => (
      <div>
        <strong>{props.label}</strong> <span>{formatValue(props.value)}</span>
      </div>
    ),
    Table: ({ props }) => (
      <table>
        <thead>
          <tr>
            {props.columns.map((column) => (
              <th key={column.key} scope="col">
                {column.label}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {props.rows.map((row, rowIndex) => (
            <tr key={rowKey(row, rowIndex)}>
              {props.columns.map((column) => (
                <td key={column.key}>{formatValue(row[column.key])}</td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    ),
  },
});

function rowKey(row: Record<string, unknown>, index: number): string {
  const id = row.id;
  return typeof id === "string" || typeof id === "number" ? String(id) : String(index);
}

function formatValue(value: unknown): string {
  if (value === null || value === undefined) {
    return "";
  }
  if (typeof value === "string") {
    return value;
  }
  if (typeof value === "number" || typeof value === "boolean" || typeof value === "bigint") {
    return String(value);
  }
  return JSON.stringify(value);
}
