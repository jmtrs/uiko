#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use uiko_capabilities::OperationParameter;
use uiko_core::AppIr;
use uiko_openapi::OpenApiTransportCatalog;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ReadRuntimePlan {
    pub operations: BTreeMap<String, ReadOperationPlan>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadOperationPlan {
    pub provider_id: String,
    pub path: String,
    pub parameters: Vec<OperationParameter>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReadPlanError {
    MissingProvider {
        logical_id: String,
        provider_id: String,
    },
    MissingOperation {
        logical_id: String,
        provider_id: String,
        external_operation_id: String,
    },
}

/// Derive the private read-only runtime plan from canonical `AppIr` plus
/// protocol transport metadata imported at the project boundary.
///
/// # Errors
///
/// Returns an internal consistency error when a compiled query no longer has a
/// matching private transport plan.
pub fn derive_read_runtime_plan(
    ir: &AppIr,
    transport: &OpenApiTransportCatalog,
) -> Result<ReadRuntimePlan, ReadPlanError> {
    let mut operations = BTreeMap::new();

    for module in &ir.modules {
        for page in &module.pages {
            for query in &page.queries {
                let provider = transport.providers.get(&query.provider_id).ok_or_else(|| {
                    ReadPlanError::MissingProvider {
                        logical_id: query.id.clone(),
                        provider_id: query.provider_id.clone(),
                    }
                })?;
                let operation = provider
                    .operations
                    .get(&query.external_operation_id)
                    .ok_or_else(|| ReadPlanError::MissingOperation {
                        logical_id: query.id.clone(),
                        provider_id: query.provider_id.clone(),
                        external_operation_id: query.external_operation_id.clone(),
                    })?;

                operations.insert(
                    query.id.clone(),
                    ReadOperationPlan {
                        provider_id: query.provider_id.clone(),
                        path: operation.path.clone(),
                        parameters: operation.parameters.clone(),
                    },
                );
            }
        }
    }

    Ok(ReadRuntimePlan { operations })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use uiko_capabilities::{OperationParameter, ParameterLocation, ValueKind, ValueShape};
    use uiko_core::{AppIr, ModuleId, ModuleIr, PageIr, QueryIr};
    use uiko_openapi::{OpenApiGetTransport, OpenApiTransportCatalog, OpenApiTransportProvider};

    use super::derive_read_runtime_plan;

    #[test]
    fn private_transport_is_keyed_by_logical_query_identity() {
        let ir = AppIr {
            app_name: "test".into(),
            spec_version: 1,
            modules: vec![ModuleIr {
                id: ModuleId::new("customers"),
                pages: vec![PageIr {
                    id: "Detail".into(),
                    route: "/customers/:customerId".into(),
                    state: vec![],
                    queries: vec![QueryIr {
                        id: "customers.Detail.query.customer".into(),
                        alias: "customer".into(),
                        provider_id: "crm".into(),
                        external_operation_id: "getCustomer".into(),
                        input: vec![],
                        output: ValueShape {
                            nullable: false,
                            kind: ValueKind::String,
                        },
                    }],
                    components: vec![],
                }],
            }],
        };
        let parameters = vec![OperationParameter {
            name: "customerId".into(),
            location: ParameterLocation::Path,
            required: true,
        }];
        let transport = OpenApiTransportCatalog {
            providers: BTreeMap::from([(
                "crm".into(),
                OpenApiTransportProvider {
                    id: "crm".into(),
                    operations: BTreeMap::from([(
                        "getCustomer".into(),
                        OpenApiGetTransport {
                            path: "/customers/{customerId}".into(),
                            parameters,
                        },
                    )]),
                },
            )]),
        };

        let plan = derive_read_runtime_plan(&ir, &transport).expect("plan");
        let operation = plan
            .operations
            .get("customers.Detail.query.customer")
            .expect("logical query");
        assert_eq!(operation.provider_id, "crm");
        assert_eq!(operation.path, "/customers/{customerId}");
    }
}
