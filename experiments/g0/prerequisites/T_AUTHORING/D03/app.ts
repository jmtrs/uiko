import type { UikoApp } from "../../../controls/t-authoring/generated/uiko-authoring.js";

const app = {
  name: "support-console",
  specVersion: 1,
  integrations: ["crm"],
  modules: [
    {
      id: "customers",
      pages: [
        {
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
            {
              id: "title",
              type: "Text",
              value: "Customers",
            },
            {
              id: "statusFilter",
              type: "Select",
              label: "Status",
              state: "status",
              options: [
                { label: "All", value: null },
                { label: "Active", value: "active" },
                { label: "Inactive", value: "inactive" },
              ],
            },
            {
              id: "table",
              type: "Table",
              binding: "customers.items",
            },
          ],
        },
      ],
    },
  ],
} satisfies UikoApp;

export default app;
