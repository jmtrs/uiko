use uiko_capabilities::CapabilityCatalog;
use uiko_compiler::compile;
use uiko_core::{Located, ModuleId, SourceId, TextSpan};
use uiko_openapi::import_openapi_provider;
use uiko_source::{AppSource, ModuleSource};
use uiko_source_jsonc::parse_page;

const CONTRACT: &str =
    include_str!("../../../experiments/fixtures/g0-support-api.openapi.json");

fn compile_page(source: &str) {
    let source_id = SourceId::new("g0/task.jsonc");
    let page = parse_page(&source_id, source).expect("task shape should parse");
    let imported = import_openapi_provider("crm", &SourceId::new("g0.openapi.json"), CONTRACT)
        .expect("frozen G0 contract should import");
    let capabilities = CapabilityCatalog {
        providers: [(String::from("crm"), imported.capabilities)]
            .into_iter()
            .collect(),
    };
    let app = Located::new(
        AppSource {
            name: "g0-expressibility".into(),
            spec_version: 1,
            modules: vec![Located::new(
                ModuleSource {
                    id: Located::new(
                        ModuleId::new("customers"),
                        TextSpan::new(SourceId::new("module"), 0, 1),
                    ),
                    pages: vec![page],
                },
                TextSpan::new(SourceId::new("module"), 0, 1),
            )],
        },
        TextSpan::new(SourceId::new("uiko.jsonc"), 0, 1),
    );

    compile(&app, &capabilities).expect("frozen task shape should compile without core edits");
}

#[test]
fn d01_customer_directory_is_expressible() {
    compile_page(
        r#"{
          "id": "CustomerList",
          "route": "/customers",
          "queries": {
            "customers": {
              "operation": "crm.listCustomers",
              "input": {}
            }
          },
          "components": [
            {
              "id": "table",
              "type": "Table",
              "binding": "customers.items",
              "columns": [
                { "label": "Name", "binding": "name" },
                { "label": "Email", "binding": "email" },
                { "label": "Status", "binding": "status" }
              ]
            }
          ]
        }"#,
    );
}

#[test]
fn d02_customer_detail_route_is_expressible() {
    compile_page(
        r#"{
          "id": "CustomerDetail",
          "route": "/customers/:customerId",
          "queries": {
            "customer": {
              "operation": "crm.getCustomer",
              "input": { "customerId": "route.customerId" }
            }
          },
          "components": [
            { "id": "name", "type": "Field", "label": "Name", "binding": "customer.name" },
            { "id": "email", "type": "Field", "label": "Email", "binding": "customer.email" },
            { "id": "status", "type": "Field", "label": "Status", "binding": "customer.status" }
          ]
        }"#,
    );
}

#[test]
fn d03_status_filter_is_expressible() {
    compile_page(
        r#"{
          "id": "CustomerList",
          "route": "/customers",
          "state": { "status": null },
          "queries": {
            "customers": {
              "operation": "crm.listCustomers",
              "input": { "status": "state.status" }
            }
          },
          "components": [
            {
              "id": "statusFilter",
              "type": "Select",
              "label": "Status",
              "state": "status",
              "options": [
                { "label": "All", "value": null },
                { "label": "Active", "value": "active" },
                { "label": "Inactive", "value": "inactive" }
              ]
            },
            {
              "id": "table",
              "type": "Table",
              "binding": "customers.items",
              "columns": [
                { "label": "Name", "binding": "name" },
                { "label": "Email", "binding": "email" },
                { "label": "Status", "binding": "status" }
              ]
            }
          ]
        }"#,
    );
}

#[test]
fn d04_nested_nullable_fields_are_expressible() {
    compile_page(
        r#"{
          "id": "CustomerDetail",
          "route": "/customers/:customerId",
          "queries": {
            "customer": {
              "operation": "crm.getCustomer",
              "input": { "customerId": "route.customerId" }
            }
          },
          "components": [
            {
              "id": "organization",
              "type": "Field",
              "label": "Organization",
              "binding": "customer.organization.name"
            },
            {
              "id": "phone",
              "type": "Field",
              "label": "Phone",
              "binding": "customer.phone",
              "fallback": "Not provided"
            }
          ]
        }"#,
    );
}

#[test]
fn d05_customer_pagination_is_expressible() {
    compile_page(
        r#"{
          "id": "CustomerList",
          "route": "/customers",
          "state": {
            "status": null,
            "page": 1
          },
          "queries": {
            "customers": {
              "operation": "crm.listCustomers",
              "input": {
                "status": "state.status",
                "page": "state.page"
              }
            }
          },
          "components": [
            {
              "id": "statusFilter",
              "type": "Select",
              "label": "Status",
              "state": "status",
              "options": [
                { "label": "All", "value": null },
                { "label": "Active", "value": "active" },
                { "label": "Inactive", "value": "inactive" }
              ]
            },
            {
              "id": "table",
              "type": "Table",
              "binding": "customers.items",
              "columns": [
                { "label": "Name", "binding": "name" },
                { "label": "Email", "binding": "email" },
                { "label": "Status", "binding": "status" }
              ]
            },
            {
              "id": "pagination",
              "type": "Pagination",
              "pageState": "page",
              "page": "customers.page",
              "pageSize": "customers.pageSize",
              "total": "customers.total"
            }
          ]
        }"#,
    );
}

#[test]
fn d06_related_customer_orders_are_expressible() {
    compile_page(
        r#"{
          "id": "CustomerDetail",
          "route": "/customers/:customerId",
          "queries": {
            "customer": {
              "operation": "crm.getCustomer",
              "input": { "customerId": "route.customerId" }
            },
            "orders": {
              "operation": "crm.listCustomerOrders",
              "input": { "customerId": "route.customerId" }
            }
          },
          "components": [
            { "id": "name", "type": "Field", "label": "Name", "binding": "customer.name" },
            {
              "id": "orders",
              "type": "Table",
              "binding": "orders.items",
              "columns": [
                { "label": "Order", "binding": "id" },
                { "label": "Status", "binding": "status" },
                { "label": "Total", "binding": "totalCents" }
              ]
            }
          ]
        }"#,
    );
}
