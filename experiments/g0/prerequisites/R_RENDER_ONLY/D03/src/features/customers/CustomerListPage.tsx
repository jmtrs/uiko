import type { Spec } from "@json-render/core";
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";

import { api } from "../../api/client";
import { JsonUi } from "../../renderer/JsonUi";

type StatusFilter = "" | "active" | "inactive";

export function CustomerListPage() {
  const [status, setStatus] = useState<StatusFilter>("");
  const customers = useQuery({
    queryKey: ["customers", status],
    queryFn: async () => {
      const query = status === "" ? {} : { status };
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
        props: { gap: 12 },
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
          columns: ["name", "email", "status"],
          rows: customers.data?.items ?? [],
        },
        children: [],
      },
    },
    state: {},
  };

  return (
    <main>
      <label>
        Status{" "}
        <select
          aria-label="Status"
          value={status}
          onChange={(event) => setStatus(event.target.value as StatusFilter)}
        >
          <option value="">All</option>
          <option value="active">Active</option>
          <option value="inactive">Inactive</option>
        </select>
      </label>
      <JsonUi spec={spec} loading={customers.isPending} />
      {customers.isError ? <p role="alert">Unable to load customers</p> : null}
    </main>
  );
}
