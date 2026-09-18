#![forbid(unsafe_code)]

use std::fmt::Write as _;

use uiko_core::{AppIr, ComponentKindIr, ScalarValue};

pub const UI_MANIFEST_SPEC_VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiManifest {
    pub spec_version: u32,
    pub app_name: String,
    pub routes: Vec<UiRouteManifest>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiRouteManifest {
    pub id: String,
    pub path: String,
    pub state: Vec<UiStateManifest>,
    pub queries: Vec<UiQueryManifest>,
    pub components: Vec<UiComponentManifest>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiStateManifest {
    pub id: String,
    pub initial: ScalarValue,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiQueryManifest {
    pub id: String,
    pub alias: String,
    pub input: Vec<UiQueryInputManifest>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiQueryInputManifest {
    pub name: String,
    pub expression: String,
    pub required: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiComponentManifest {
    pub id: String,
    pub kind: UiComponentKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UiComponentKind {
    Text {
        value: String,
    },
    Field {
        label: String,
        binding: String,
        fallback: Option<String>,
    },
    Table {
        binding: String,
        columns: Vec<UiTableColumn>,
    },
    Select {
        label: String,
        state: String,
        options: Vec<UiSelectOption>,
    },
    Pagination {
        page_state: String,
        page_binding: String,
        page_size_binding: String,
        total_binding: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiTableColumn {
    pub label: String,
    pub binding: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiSelectOption {
    pub label: String,
    pub value: ScalarValue,
}

#[must_use]
pub fn derive_ui_manifest(ir: &AppIr) -> UiManifest {
    UiManifest {
        spec_version: UI_MANIFEST_SPEC_VERSION,
        app_name: ir.app_name.clone(),
        routes: ir
            .modules
            .iter()
            .flat_map(|module| {
                module.pages.iter().map(|page| {
                    let route_id = format!("{}.{}", module.id.as_str(), page.id);
                    UiRouteManifest {
                        id: route_id.clone(),
                        path: page.route.clone(),
                        state: page
                            .state
                            .iter()
                            .map(|state| UiStateManifest {
                                id: state.id.clone(),
                                initial: state.initial.clone(),
                            })
                            .collect(),
                        queries: page
                            .queries
                            .iter()
                            .map(|query| UiQueryManifest {
                                id: query.id.clone(),
                                alias: query.alias.clone(),
                                input: query
                                    .input
                                    .iter()
                                    .map(|binding| UiQueryInputManifest {
                                        name: binding.name.clone(),
                                        expression: binding.expression.clone(),
                                        required: binding.required,
                                    })
                                    .collect(),
                            })
                            .collect(),
                        components: page
                            .components
                            .iter()
                            .map(|component| UiComponentManifest {
                                id: format!("{route_id}.{}", component.id),
                                kind: match &component.kind {
                                    ComponentKindIr::Text { value } => UiComponentKind::Text {
                                        value: value.clone(),
                                    },
                                    ComponentKindIr::Field {
                                        label,
                                        binding,
                                        fallback,
                                    } => UiComponentKind::Field {
                                        label: label.clone(),
                                        binding: binding.clone(),
                                        fallback: fallback.clone(),
                                    },
                                    ComponentKindIr::Table { binding, columns } => {
                                        UiComponentKind::Table {
                                            binding: binding.clone(),
                                            columns: columns
                                                .iter()
                                                .map(|column| UiTableColumn {
                                                    label: column.label.clone(),
                                                    binding: column.binding.clone(),
                                                })
                                                .collect(),
                                        }
                                    }
                                    ComponentKindIr::Select {
                                        label,
                                        state,
                                        options,
                                    } => UiComponentKind::Select {
                                        label: label.clone(),
                                        state: state.clone(),
                                        options: options
                                            .iter()
                                            .map(|option| UiSelectOption {
                                                label: option.label.clone(),
                                                value: option.value.clone(),
                                            })
                                            .collect(),
                                    },
                                    ComponentKindIr::Pagination {
                                        page_state,
                                        page_binding,
                                        page_size_binding,
                                        total_binding,
                                    } => UiComponentKind::Pagination {
                                        page_state: page_state.clone(),
                                        page_binding: page_binding.clone(),
                                        page_size_binding: page_size_binding.clone(),
                                        total_binding: total_binding.clone(),
                                    },
                                },
                            })
                            .collect(),
                    }
                })
            })
            .collect(),
    }
}

impl UiManifest {
    #[must_use]
    pub fn to_json_pretty(&self) -> String {
        let mut output = String::new();
        writeln!(output, "{{").expect("writing to String cannot fail");
        writeln!(output, "  \"specVersion\": {},", self.spec_version)
            .expect("writing to String cannot fail");
        writeln!(output, "  \"app\": \"{}\",", json_escape(&self.app_name))
            .expect("writing to String cannot fail");
        writeln!(output, "  \"routes\": [").expect("writing to String cannot fail");

        for (route_index, route) in self.routes.iter().enumerate() {
            writeln!(output, "    {{").expect("writing to String cannot fail");
            writeln!(output, "      \"id\": \"{}\",", json_escape(&route.id))
                .expect("writing to String cannot fail");
            writeln!(output, "      \"path\": \"{}\",", json_escape(&route.path))
                .expect("writing to String cannot fail");
            writeln!(output, "      \"state\": {{").expect("writing to String cannot fail");
            for (index, state) in route.state.iter().enumerate() {
                write!(output, "        \"{}\": ", json_escape(&state.id),)
                    .expect("writing to String cannot fail");
                write_scalar(&mut output, &state.initial);
                if index + 1 < route.state.len() {
                    output.push(',');
                }
                output.push('\n');
            }
            writeln!(output, "      }},").expect("writing to String cannot fail");
            writeln!(output, "      \"queries\": [").expect("writing to String cannot fail");
            for (query_index, query) in route.queries.iter().enumerate() {
                write_query(&mut output, query, 8);
                if query_index + 1 < route.queries.len() {
                    output.push(',');
                }
                output.push('\n');
            }
            writeln!(output, "      ],").expect("writing to String cannot fail");
            writeln!(output, "      \"components\": [").expect("writing to String cannot fail");

            for (component_index, component) in route.components.iter().enumerate() {
                write_component(&mut output, component, 8);
                if component_index + 1 < route.components.len() {
                    output.push(',');
                }
                output.push('\n');
            }

            writeln!(output, "      ]").expect("writing to String cannot fail");
            write!(output, "    }}").expect("writing to String cannot fail");
            if route_index + 1 < self.routes.len() {
                output.push(',');
            }
            output.push('\n');
        }

        writeln!(output, "  ]").expect("writing to String cannot fail");
        output.push('}');
        output
    }
}

fn write_query(output: &mut String, query: &UiQueryManifest, indent: usize) {
    let pad = " ".repeat(indent);
    write!(
        output,
        "{pad}{{\"id\":\"{}\",\"alias\":\"{}\",\"input\":[",
        json_escape(&query.id),
        json_escape(&query.alias)
    )
    .expect("writing to String cannot fail");

    for (index, binding) in query.input.iter().enumerate() {
        write!(
            output,
            "{{\"name\":\"{}\",\"expression\":\"{}\",\"required\":{}}}",
            json_escape(&binding.name),
            json_escape(&binding.expression),
            binding.required
        )
        .expect("writing to String cannot fail");
        if index + 1 < query.input.len() {
            output.push(',');
        }
    }
    output.push_str("]}");
}

fn write_component(output: &mut String, component: &UiComponentManifest, indent: usize) {
    let pad = " ".repeat(indent);
    write!(output, "{pad}{{\"id\":\"{}\",", json_escape(&component.id))
        .expect("writing to String cannot fail");

    match &component.kind {
        UiComponentKind::Text { value } => {
            write!(
                output,
                "\"kind\":\"Text\",\"value\":\"{}\"}}",
                json_escape(value)
            )
            .expect("writing to String cannot fail");
        }
        UiComponentKind::Field {
            label,
            binding,
            fallback,
        } => {
            write!(
                output,
                "\"kind\":\"Field\",\"label\":\"{}\",\"binding\":\"{}\"",
                json_escape(label),
                json_escape(binding)
            )
            .expect("writing to String cannot fail");
            if let Some(fallback) = fallback {
                write!(output, ",\"fallback\":\"{}\"", json_escape(fallback))
                    .expect("writing to String cannot fail");
            }
            output.push('}');
        }
        UiComponentKind::Table { binding, columns } => {
            write!(
                output,
                "\"kind\":\"Table\",\"binding\":\"{}\",\"columns\":[",
                json_escape(binding)
            )
            .expect("writing to String cannot fail");
            for (index, column) in columns.iter().enumerate() {
                write!(
                    output,
                    "{{\"label\":\"{}\",\"binding\":\"{}\"}}",
                    json_escape(&column.label),
                    json_escape(&column.binding)
                )
                .expect("writing to String cannot fail");
                if index + 1 < columns.len() {
                    output.push(',');
                }
            }
            output.push_str("]}");
        }
        UiComponentKind::Select {
            label,
            state,
            options,
        } => {
            write!(
                output,
                "\"kind\":\"Select\",\"label\":\"{}\",\"state\":\"{}\",\"options\":[",
                json_escape(label),
                json_escape(state)
            )
            .expect("writing to String cannot fail");
            for (index, option) in options.iter().enumerate() {
                write!(
                    output,
                    "{{\"label\":\"{}\",\"value\":",
                    json_escape(&option.label)
                )
                .expect("writing to String cannot fail");
                write_scalar(output, &option.value);
                output.push('}');
                if index + 1 < options.len() {
                    output.push(',');
                }
            }
            output.push_str("]}");
        }
        UiComponentKind::Pagination {
            page_state,
            page_binding,
            page_size_binding,
            total_binding,
        } => {
            write!(
                output,
                "\"kind\":\"Pagination\",\"pageState\":\"{}\",\"page\":\"{}\",\"pageSize\":\"{}\",\"total\":\"{}\"}}",
                json_escape(page_state),
                json_escape(page_binding),
                json_escape(page_size_binding),
                json_escape(total_binding)
            )
            .expect("writing to String cannot fail");
        }
    }
}

fn write_scalar(output: &mut String, value: &ScalarValue) {
    match value {
        ScalarValue::Null => output.push_str("null"),
        ScalarValue::Boolean(value) => output.push_str(if *value { "true" } else { "false" }),
        ScalarValue::Number(value) => output.push_str(value),
        ScalarValue::String(value) => {
            write!(output, "\"{}\"", json_escape(value)).expect("writing to String cannot fail");
        }
    }
}

fn json_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());

    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            control if control <= '\u{001f}' => {
                write!(escaped, "\\u{:04x}", u32::from(control))
                    .expect("writing JSON escape to String cannot fail");
            }
            other => escaped.push(other),
        }
    }

    escaped
}

#[cfg(test)]
mod tests {
    use uiko_core::{
        AppIr, ComponentIr, ComponentKindIr, ModuleId, ModuleIr, PageIr, ScalarValue, StateIr,
    };

    use super::{UI_MANIFEST_SPEC_VERSION, derive_ui_manifest};

    fn fixture_ir() -> AppIr {
        AppIr {
            app_name: "support-console".into(),
            spec_version: 1,
            modules: vec![ModuleIr {
                id: ModuleId::new("customers"),
                pages: vec![PageIr {
                    id: "CustomerDetail".into(),
                    route: "/customers/:customerId".into(),
                    state: vec![StateIr {
                        id: "status".into(),
                        initial: ScalarValue::Null,
                    }],
                    queries: vec![uiko_core::QueryIr {
                        id: "customers.CustomerDetail.query.customer".into(),
                        alias: "customer".into(),
                        provider_id: "crm".into(),
                        external_operation_id: "getCustomer".into(),
                        input: vec![uiko_core::QueryInputIr {
                            name: "customerId".into(),
                            expression: "route.customerId".into(),
                            required: true,
                        }],
                        output: uiko_capabilities::ValueShape {
                            nullable: false,
                            kind: uiko_capabilities::ValueKind::Object(
                                std::collections::BTreeMap::new(),
                            ),
                        },
                    }],
                    components: vec![
                        ComponentIr {
                            id: "title".into(),
                            kind: ComponentKindIr::Text {
                                value: "Customer".into(),
                            },
                        },
                        ComponentIr {
                            id: "name".into(),
                            kind: ComponentKindIr::Field {
                                label: "Name".into(),
                                binding: "customer.name".into(),
                                fallback: Some("Not provided".into()),
                            },
                        },
                    ],
                }],
            }],
        }
    }

    #[test]
    fn public_ids_derive_from_symbolic_identity_not_position() {
        let manifest = derive_ui_manifest(&fixture_ir());

        assert_eq!(manifest.spec_version, UI_MANIFEST_SPEC_VERSION);
        assert_eq!(manifest.routes[0].id, "customers.CustomerDetail");
        assert_eq!(manifest.routes[0].state[0].id, "status");
        assert_eq!(
            manifest.routes[0].queries[0].id,
            "customers.CustomerDetail.query.customer"
        );
        assert_eq!(
            manifest.routes[0].components[0].id,
            "customers.CustomerDetail.title"
        );
        assert_eq!(
            manifest.routes[0].components[1].id,
            "customers.CustomerDetail.name"
        );
    }

    #[test]
    fn manifest_json_is_deterministic_and_keeps_transport_private() {
        let first = derive_ui_manifest(&fixture_ir()).to_json_pretty();
        let second = derive_ui_manifest(&fixture_ir()).to_json_pretty();

        assert_eq!(first, second);
        assert!(first.contains(r#"\"state\": {"#));
        assert!(first.contains(r#"\"fallback\":\"Not provided\""#));
        assert!(!first.contains("provider_id"));
        assert!(!first.contains("external_operation_id"));
    }
}
