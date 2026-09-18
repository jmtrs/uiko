import type { ProjectSpec } from "../harness/facade.mjs";

export default {
  name: "support-console",
  specVersion: 1,
  modules: [
    {
      file: "features/customers/module.jsonc",
      id: "customers",
      pages: [
        {
          file: "features/customers/list.jsonc",
          id: "CustomerList",
          route: "/customers",
          state: {
            status: null,
          },
          queries: {
            customers: {
              operation: "crm.listCustomers",
              input: {
                status: "state.status",
              },
            },
          },
          components: [
            { id: "title", type: "Text", value: "Customers" },
            {
              id: "status",
              type: "Select",
              label: "Status",
              state: "status",
              options: [
                { label: "All", value: null },
                { label: "Active", value: "active" },
                { label: "Inactive", value: "inactive" },
              ],
            },
            { id: "table", type: "Table", binding: "customers.items" },
          ],
        },
      ],
    },
  ],
} satisfies ProjectSpec;
