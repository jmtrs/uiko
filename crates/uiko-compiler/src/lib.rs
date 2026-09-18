#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use uiko_capabilities::{CapabilityCatalog, QueryOperation, ValueShape};
use uiko_core::{
    AppIr, ComponentIr, ComponentKindIr, Diagnostic, Located, ModuleIr, PageIr, QueryInputIr,
    QueryIr, ScalarValue, SelectOptionIr, Severity, StateIr, TableColumnIr,
};
use uiko_source::{
    AppSource, ComponentKindSource, ComponentSource, PageSource, QuerySource, StateSource,
};

pub const SUPPORTED_SPEC_VERSION: u32 = 1;

/// Compile located source DTOs plus normalized external capabilities into canonical IR.
///
/// Parsing, filesystem I/O and protocol-specific contract parsing intentionally
/// live outside this pure semantic boundary.
///
/// # Errors
///
/// Returns stable diagnostics when source-level semantic invariants fail.
pub fn compile(
    source: &Located<AppSource>,
    capabilities: &CapabilityCatalog,
) -> Result<AppIr, Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();

    if source.value.spec_version != SUPPORTED_SPEC_VERSION {
        diagnostics.push(Diagnostic {
            code: "UIKO1001",
            severity: Severity::Error,
            message: format!(
                "unsupported specVersion {}; expected {}",
                source.value.spec_version, SUPPORTED_SPEC_VERSION
            ),
            span: source.span.clone(),
        });
    }

    let mut module_ids = BTreeSet::new();
    for module in &source.value.modules {
        if !module_ids.insert(module.value.id.value.clone()) {
            diagnostics.push(Diagnostic::error(
                "UIKO1101",
                format!("duplicate module `{}`", module.value.id.value),
                module.value.id.span.clone(),
            ));
        }

        let mut page_ids = BTreeSet::new();
        for page in &module.value.pages {
            if !page_ids.insert(page.value.id.value.clone()) {
                diagnostics.push(Diagnostic::error(
                    "UIKO1102",
                    format!(
                        "duplicate page `{}` in module `{}`",
                        page.value.id.value, module.value.id.value
                    ),
                    page.value.id.span.clone(),
                ));
            }

            let mut state_ids = BTreeSet::new();
            for state in &page.value.state {
                if !state_ids.insert(state.value.id.value.clone()) {
                    diagnostics.push(Diagnostic::error(
                        "UIKO1104",
                        format!(
                            "duplicate state key `{}` in page `{}`",
                            state.value.id.value, page.value.id.value
                        ),
                        state.value.id.span.clone(),
                    ));
                }
            }

            let mut component_ids = BTreeSet::new();
            for component in &page.value.components {
                if !component_ids.insert(component.value.id.value.clone()) {
                    diagnostics.push(Diagnostic::error(
                        "UIKO1103",
                        format!(
                            "duplicate component `{}` in page `{}`",
                            component.value.id.value, page.value.id.value
                        ),
                        component.value.id.span.clone(),
                    ));
                }
            }
        }
    }

    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    let mut modules = Vec::with_capacity(source.value.modules.len());
    for module in &source.value.modules {
        let mut pages = Vec::with_capacity(module.value.pages.len());
        for page in &module.value.pages {
            match lower_page(module.value.id.value.as_str(), &page.value, capabilities) {
                Ok(lowered) => pages.push(lowered),
                Err(errors) => diagnostics.extend(errors),
            }
        }
        modules.push(ModuleIr {
            id: module.value.id.value.clone(),
            pages,
        });
    }

    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    Ok(AppIr {
        app_name: source.value.name.clone(),
        spec_version: source.value.spec_version,
        modules,
    })
}

fn lower_page(
    module_id: &str,
    page: &PageSource,
    capabilities: &CapabilityCatalog,
) -> Result<PageIr, Vec<Diagnostic>> {
    let route_parameters = route_parameters(&page.route.value);
    let state = page
        .state
        .iter()
        .map(|entry| (entry.value.id.value.clone(), &entry.value))
        .collect::<BTreeMap<_, _>>();
    let mut diagnostics = Vec::new();
    let mut query_aliases = BTreeMap::<String, &QueryOperation>::new();
    let mut queries = Vec::with_capacity(page.queries.len());

    for query in &page.queries {
        let alias = query.value.id.value.as_str();
        if query_aliases.contains_key(alias) {
            diagnostics.push(Diagnostic::error(
                "UIKO2100",
                format!("duplicate query `{alias}`"),
                query.value.id.span.clone(),
            ));
            continue;
        }

        match lower_query(
            module_id,
            page,
            &query.value,
            capabilities,
            &route_parameters,
            &state,
        ) {
            Ok((lowered, operation)) => {
                query_aliases.insert(query.value.id.value.clone(), operation);
                queries.push(lowered);
            }
            Err(errors) => diagnostics.extend(errors),
        }
    }

    for component in &page.components {
        validate_component(
            &component.value,
            &component.span,
            &query_aliases,
            &state,
            &mut diagnostics,
        );
    }

    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    Ok(PageIr {
        id: page.id.value.clone(),
        route: page.route.value.clone(),
        state: page
            .state
            .iter()
            .map(|entry| lower_state(&entry.value))
            .collect(),
        queries,
        components: page
            .components
            .iter()
            .map(|component| lower_component(&component.value))
            .collect(),
    })
}

fn lower_query<'a>(
    module_id: &str,
    page: &PageSource,
    query: &QuerySource,
    capabilities: &'a CapabilityCatalog,
    route_parameters: &BTreeSet<String>,
    state: &BTreeMap<String, &StateSource>,
) -> Result<(QueryIr, &'a QueryOperation), Vec<Diagnostic>> {
    let Some((provider_id, operation_id)) = parse_operation_reference(&query.operation.value) else {
        return Err(vec![Diagnostic::error(
            "UIKO2103",
            format!(
                "operation reference `{}` must be `integration.operationId`",
                query.operation.value
            ),
            query.operation.span.clone(),
        )]);
    };

    let Some(provider) = capabilities.providers.get(provider_id) else {
        return Err(vec![Diagnostic::error(
            "UIKO2102",
            format!("unknown integration `{provider_id}`"),
            query.operation.span.clone(),
        )]);
    };
    let Some(operation) = provider.operations.get(operation_id) else {
        return Err(vec![Diagnostic::error(
            "UIKO2104",
            format!("unknown operation `{operation_id}` on integration `{provider_id}`"),
            query.operation.span.clone(),
        )]);
    };

    let diagnostics = validate_query_input(query, operation, route_parameters, state);
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    let logical_id = format!(
        "{module_id}.{}.query.{}",
        page.id.value, query.id.value
    );
    let input = query
        .input
        .iter()
        .filter_map(|binding| {
            let required = operation
                .parameters
                .iter()
                .find(|parameter| parameter.name == binding.value.name.value)
                .map(|parameter| parameter.required)?;
            Some(QueryInputIr {
                name: binding.value.name.value.clone(),
                expression: binding.value.expression.value.clone(),
                required,
            })
        })
        .collect();

    Ok((
        QueryIr {
            id: logical_id,
            alias: query.id.value.clone(),
            provider_id: provider_id.to_string(),
            external_operation_id: operation_id.to_string(),
            input,
            output: operation.output.clone(),
        },
        operation,
    ))
}

fn validate_query_input(
    query: &QuerySource,
    operation: &QueryOperation,
    route_parameters: &BTreeSet<String>,
    state: &BTreeMap<String, &StateSource>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut supplied = BTreeSet::new();

    for binding in &query.input {
        let name = binding.value.name.value.as_str();
        if !supplied.insert(name.to_string()) {
            diagnostics.push(Diagnostic::error(
                "UIKO2108",
                format!("query input `{name}` is bound more than once"),
                binding.value.name.span.clone(),
            ));
            continue;
        }

        let parameter = operation
            .parameters
            .iter()
            .find(|parameter| parameter.name == name);

        if parameter.is_none() {
            diagnostics.push(Diagnostic::error(
                "UIKO2105",
                format!("operation has no input parameter `{name}`"),
                binding.value.name.span.clone(),
            ));
        }

        let expression = binding.value.expression.value.as_str();
        if let Some(route_parameter) = expression.strip_prefix("route.") {
            if !route_parameters.contains(route_parameter) {
                diagnostics.push(Diagnostic::error(
                    "UIKO2107",
                    format!("route has no parameter `{route_parameter}`"),
                    binding.value.expression.span.clone(),
                ));
            }
            continue;
        }

        if let Some(state_key) = expression.strip_prefix("state.") {
            let Some(state_entry) = state.get(state_key) else {
                diagnostics.push(Diagnostic::error(
                    "UIKO2112",
                    format!("page has no state key `{state_key}`"),
                    binding.value.expression.span.clone(),
                ));
                continue;
            };
            if parameter.is_some_and(|parameter| parameter.required)
                && matches!(state_entry.initial.value, ScalarValue::Null)
            {
                diagnostics.push(Diagnostic::error(
                    "UIKO2113",
                    format!(
                        "required operation input `{name}` cannot bind to initially null state `{state_key}`"
                    ),
                    binding.value.expression.span.clone(),
                ));
            }
            continue;
        }

        diagnostics.push(Diagnostic::error(
            "UIKO2106",
            format!(
                "unsupported input binding `{expression}`; expected route.<param> or state.<key>"
            ),
            binding.value.expression.span.clone(),
        ));
    }

    for parameter in operation
        .parameters
        .iter()
        .filter(|parameter| parameter.required)
    {
        if !supplied.contains(&parameter.name) {
            diagnostics.push(Diagnostic::error(
                "UIKO2109",
                format!(
                    "required operation input `{}` is not bound by query `{}`",
                    parameter.name, query.id.value
                ),
                query.operation.span.clone(),
            ));
        }
    }

    diagnostics
}

fn validate_component(
    component: &ComponentSource,
    span: &uiko_core::TextSpan,
    queries: &BTreeMap<String, &QueryOperation>,
    state: &BTreeMap<String, &StateSource>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match &component.kind {
        ComponentKindSource::Text { .. } => {}
        ComponentKindSource::Field { binding, .. } => {
            validate_component_binding(binding, queries, span, diagnostics);
        }
        ComponentKindSource::Table { binding, columns } => {
            let Some(shape) = resolve_component_binding(binding, queries, span, diagnostics) else {
                return;
            };
            let Some(item) = shape.array_item() else {
                diagnostics.push(Diagnostic::error(
                    "UIKO2114",
                    format!("Table binding `{binding}` must resolve to an array"),
                    span.clone(),
                ));
                return;
            };
            for column in columns {
                if item.at_path(column.binding.split('.')).is_none() {
                    diagnostics.push(Diagnostic::error(
                        "UIKO2115",
                        format!(
                            "Table column binding `{}` does not exist on items from `{binding}`",
                            column.binding
                        ),
                        span.clone(),
                    ));
                }
            }
        }
        ComponentKindSource::Select {
            state: state_key, ..
        } => {
            if !state.contains_key(state_key) {
                diagnostics.push(Diagnostic::error(
                    "UIKO2116",
                    format!("Select references unknown state key `{state_key}`"),
                    span.clone(),
                ));
            }
        }
        ComponentKindSource::Pagination {
            page_state,
            page_binding,
            page_size_binding,
            total_binding,
        } => {
            match state.get(page_state) {
                Some(entry) if matches!(entry.initial.value, ScalarValue::Number(_)) => {}
                Some(_) => diagnostics.push(Diagnostic::error(
                    "UIKO2117",
                    format!("Pagination page state `{page_state}` must be numeric"),
                    span.clone(),
                )),
                None => diagnostics.push(Diagnostic::error(
                    "UIKO2116",
                    format!("Pagination references unknown state key `{page_state}`"),
                    span.clone(),
                )),
            }

            for binding in [page_binding, page_size_binding, total_binding] {
                let Some(shape) = resolve_component_binding(binding, queries, span, diagnostics)
                else {
                    continue;
                };
                if !shape.is_numeric() {
                    diagnostics.push(Diagnostic::error(
                        "UIKO2118",
                        format!("Pagination binding `{binding}` must resolve to a number"),
                        span.clone(),
                    ));
                }
            }
        }
    }
}

fn validate_component_binding(
    binding: &str,
    queries: &BTreeMap<String, &QueryOperation>,
    span: &uiko_core::TextSpan,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let _ = resolve_component_binding(binding, queries, span, diagnostics);
}

fn resolve_component_binding<'a>(
    binding: &str,
    queries: &BTreeMap<String, &'a QueryOperation>,
    span: &uiko_core::TextSpan,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<&'a ValueShape> {
    let mut segments = binding.split('.');
    let alias = segments.next()?;
    let Some(operation) = queries.get(alias) else {
        diagnostics.push(Diagnostic::error(
            "UIKO2110",
            format!("binding `{binding}` references unknown query `{alias}`"),
            span.clone(),
        ));
        return None;
    };

    let path: Vec<_> = segments.collect();
    let shape = operation.output.at_path(path.iter().copied());
    if shape.is_none() {
        diagnostics.push(Diagnostic::error(
            "UIKO2111",
            format!("binding `{binding}` does not exist in output of query `{alias}`"),
            span.clone(),
        ));
    }
    shape
}

fn parse_operation_reference(reference: &str) -> Option<(&str, &str)> {
    let (provider, operation) = reference.split_once('.')?;
    if provider.is_empty() || operation.is_empty() || operation.contains('.') {
        return None;
    }
    Some((provider, operation))
}

fn route_parameters(route: &str) -> BTreeSet<String> {
    route
        .split('/')
        .filter_map(|segment| segment.strip_prefix(':'))
        .filter(|segment| !segment.is_empty())
        .map(str::to_string)
        .collect()
}

fn lower_state(state: &StateSource) -> StateIr {
    StateIr {
        id: state.id.value.clone(),
        initial: state.initial.value.clone(),
    }
}

fn lower_component(component: &ComponentSource) -> ComponentIr {
    ComponentIr {
        id: component.id.value.clone(),
        kind: match &component.kind {
            ComponentKindSource::Text { value } => ComponentKindIr::Text {
                value: value.clone(),
            },
            ComponentKindSource::Field {
                label,
                binding,
                fallback,
            } => ComponentKindIr::Field {
                label: label.clone(),
                binding: binding.clone(),
                fallback: fallback.clone(),
            },
            ComponentKindSource::Table { binding, columns } => ComponentKindIr::Table {
                binding: binding.clone(),
                columns: columns
                    .iter()
                    .map(|column| TableColumnIr {
                        label: column.label.clone(),
                        binding: column.binding.clone(),
                    })
                    .collect(),
            },
            ComponentKindSource::Select {
                label,
                state,
                options,
            } => ComponentKindIr::Select {
                label: label.clone(),
                state: state.clone(),
                options: options
                    .iter()
                    .map(|option| SelectOptionIr {
                        label: option.label.clone(),
                        value: option.value.clone(),
                    })
                    .collect(),
            },
            ComponentKindSource::Pagination {
                page_state,
                page_binding,
                page_size_binding,
                total_binding,
            } => ComponentKindIr::Pagination {
                page_state: page_state.clone(),
                page_binding: page_binding.clone(),
                page_size_binding: page_size_binding.clone(),
                total_binding: total_binding.clone(),
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use uiko_capabilities::{
        CapabilityCatalog, CapabilityProvider, ObjectField, OperationParameter, ParameterLocation,
        QueryOperation, ValueKind, ValueShape,
    };
    use uiko_core::{Located, ModuleId, ScalarValue, SourceId, TextSpan};
    use uiko_source::{
        AppSource, ComponentKindSource, ComponentSource, InputBindingSource, ModuleSource,
        PageSource, QuerySource, StateSource,
    };

    use super::{SUPPORTED_SPEC_VERSION, compile};

    fn at<T>(value: T, source: &str) -> Located<T> {
        Located::new(value, TextSpan::new(SourceId::new(source), 0, 10))
    }

    fn catalog() -> CapabilityCatalog {
        let output = ValueShape {
            nullable: false,
            kind: ValueKind::Object(BTreeMap::from([
                (
                    "name".into(),
                    ObjectField {
                        required: true,
                        value: ValueShape {
                            nullable: false,
                            kind: ValueKind::String,
                        },
                    },
                ),
                (
                    "page".into(),
                    ObjectField {
                        required: true,
                        value: ValueShape {
                            nullable: false,
                            kind: ValueKind::Integer,
                        },
                    },
                ),
            ])),
        };
        CapabilityCatalog {
            providers: BTreeMap::from([(
                "crm".into(),
                CapabilityProvider {
                    id: "crm".into(),
                    operations: BTreeMap::from([(
                        "getCustomer".into(),
                        QueryOperation {
                            external_id: "getCustomer".into(),
                            parameters: vec![
                                OperationParameter {
                                    name: "customerId".into(),
                                    location: ParameterLocation::Path,
                                    required: true,
                                },
                                OperationParameter {
                                    name: "status".into(),
                                    location: ParameterLocation::Query,
                                    required: false,
                                },
                            ],
                            output,
                        },
                    )]),
                },
            )]),
        }
    }

    fn app() -> Located<AppSource> {
        at(
            AppSource {
                name: "support-console".into(),
                spec_version: SUPPORTED_SPEC_VERSION,
                modules: vec![at(
                    ModuleSource {
                        id: at(
                            ModuleId::new("customers"),
                            "features/customers/module.jsonc",
                        ),
                        pages: vec![at(
                            PageSource {
                                id: at("CustomerDetail".into(), "features/customers/detail.jsonc"),
                                route: at(
                                    "/customers/:customerId".into(),
                                    "features/customers/detail.jsonc",
                                ),
                                state: vec![],
                                queries: vec![at(
                                    QuerySource {
                                        id: at(
                                            "customer".into(),
                                            "features/customers/detail.jsonc",
                                        ),
                                        operation: at(
                                            "crm.getCustomer".into(),
                                            "features/customers/detail.jsonc",
                                        ),
                                        input: vec![at(
                                            InputBindingSource {
                                                name: at(
                                                    "customerId".into(),
                                                    "features/customers/detail.jsonc",
                                                ),
                                                expression: at(
                                                    "route.customerId".into(),
                                                    "features/customers/detail.jsonc",
                                                ),
                                            },
                                            "features/customers/detail.jsonc",
                                        )],
                                    },
                                    "features/customers/detail.jsonc",
                                )],
                                components: vec![at(
                                    ComponentSource {
                                        id: at("name".into(), "features/customers/detail.jsonc"),
                                        kind: ComponentKindSource::Field {
                                            label: "Name".into(),
                                            binding: "customer.name".into(),
                                            fallback: None,
                                        },
                                    },
                                    "features/customers/detail.jsonc",
                                )],
                            },
                            "features/customers/detail.jsonc",
                        )],
                    },
                    "features/customers/module.jsonc",
                )],
            },
            "uiko.jsonc",
        )
    }

    #[test]
    fn route_input_and_output_binding_compile_to_logical_query() {
        let ir = compile(&app(), &catalog()).expect("valid source");
        let query = &ir.modules[0].pages[0].queries[0];
        assert_eq!(query.id, "customers.CustomerDetail.query.customer");
        assert_eq!(query.provider_id, "crm");
        assert_eq!(query.external_operation_id, "getCustomer");
        assert_eq!(query.input[0].expression, "route.customerId");
        assert!(query.input[0].required);
    }

    #[test]
    fn optional_state_input_compiles_and_is_marked_optional() {
        let mut source = app();
        source.value.modules[0].value.pages[0].value.state.push(at(
            StateSource {
                id: at("status".into(), "features/customers/detail.jsonc"),
                initial: at(ScalarValue::Null, "features/customers/detail.jsonc"),
            },
            "features/customers/detail.jsonc",
        ));
        source.value.modules[0].value.pages[0].value.queries[0]
            .value
            .input
            .push(at(
                InputBindingSource {
                    name: at("status".into(), "features/customers/detail.jsonc"),
                    expression: at("state.status".into(), "features/customers/detail.jsonc"),
                },
                "features/customers/detail.jsonc",
            ));

        let ir = compile(&source, &catalog()).expect("optional state input");
        assert!(!ir.modules[0].pages[0].queries[0].input[1].required);
    }

    #[test]
    fn unknown_state_input_fails_before_browser_execution() {
        let mut source = app();
        source.value.modules[0].value.pages[0].value.queries[0]
            .value
            .input
            .push(at(
                InputBindingSource {
                    name: at("status".into(), "features/customers/detail.jsonc"),
                    expression: at("state.missing".into(), "features/customers/detail.jsonc"),
                },
                "features/customers/detail.jsonc",
            ));

        let diagnostics = compile(&source, &catalog()).unwrap_err();
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "UIKO2112")
        );
    }

    #[test]
    fn unknown_operation_fails_before_browser_execution() {
        let mut source = app();
        source.value.modules[0].value.pages[0].value.queries[0]
            .value
            .operation
            .value = "crm.getCustmer".into();

        let diagnostics = compile(&source, &catalog()).unwrap_err();
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "UIKO2104")
        );
    }

    #[test]
    fn missing_route_parameter_binding_fails() {
        let mut source = app();
        source.value.modules[0].value.pages[0].value.queries[0]
            .value
            .input[0]
            .value
            .expression
            .value = "route.id".into();

        let diagnostics = compile(&source, &catalog()).unwrap_err();
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "UIKO2107")
        );
    }

    #[test]
    fn invalid_output_binding_fails() {
        let mut source = app();
        source.value.modules[0].value.pages[0].value.components[0]
            .value
            .kind = ComponentKindSource::Field {
            label: "Broken".into(),
            binding: "customer.missing".into(),
            fallback: None,
        };

        let diagnostics = compile(&source, &catalog()).unwrap_err();
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "UIKO2111")
        );
    }
}
