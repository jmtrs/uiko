import type { Spec } from "@json-render/core";
import { JSONUIProvider, Renderer } from "@json-render/react";
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";

import { api } from "../../api/client";
import { registry } from "../../ui/catalog";

type StatusFilter = "all" | "active" | "inactive";

export function CustomerListPage() {
  const [status, setStatus] = useState<StatusFilter>("all");
  const customers = useQuery({
    queryKey: ["customers", status],
    queryFn: async () => {
      const query = status === "all" ? {} : { status };
      const { data, error } = await api.GET("/customers", {
        params: { query },
      });
      if (error !== undefined) {
        throw new Error("Unable to load customers");
      }
      return data;
    },
  });

  const spec: Spec = {
    root: "root",
    elements: {
      root: {
        type: "Stack",
        props: {},
        children: ["title", "table"],
      },
      title: {
        type: "Text",
        props: { value: "Customers" },
        children: [],
      },
      table: {
        type: "Table",
        props: {
          columns: [
            { label: "Name", key: "name" },
            { label: "Email", key: "email" },
            { label: "Status", key: "status" },
          ],
          rows:
            customers.data?.items.map((customer) => ({
              name: customer.name,
              email: customer.email,
              status: customer.status,
            })) ?? [],
        },
        children: [],
      },
    },
  };

  return (
    <main>
      <label>
        Status
        <select
          aria-label="Status"
          value={status}
          onChange={(event) => setStatus(event.target.value as StatusFilter)}
        >
          <option value="all">All</option>
          <option value="active">Active</option>
          <option value="inactive">Inactive</option>
        </select>
      </label>
      {customers.isPending ? (
        <p role="status">Loading…</p>
      ) : customers.isError ? (
        <p role="alert">Unable to load customers</p>
      ) : (
        <JSONUIProvider registry={registry} initialState={{}}>
          <Renderer spec={spec} registry={registry} />
        </JSONUIProvider>
      )}
    </main>
  );
}
