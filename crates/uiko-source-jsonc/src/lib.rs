#![forbid(unsafe_code)]

use std::collections::BTreeSet;

use jsonc_parser::{
    CollectOptions, ParseOptions,
    ast::{Object, ObjectProp, Value},
    common::{Range, Ranged},
    parse_to_ast,
};
use uiko_core::{Diagnostic, Located, ScalarValue, SourceId, TextSpan};
use uiko_source::{
    AppConfigSource, ComponentKindSource, ComponentSource, InputBindingSource,
    IntegrationConfigSource, ModuleConfigSource, PageSource, QuerySource, SelectOptionSource,
    StateSource, TableColumnSource,
};

const APP_FIELDS: [&str; 4] = ["name", "specVersion", "modules", "integrations"];
const MODULE_FIELDS: [&str; 2] = ["id", "pages"];
const INTEGRATION_FIELDS: [&str; 3] = ["id", "adapter", "contract"];
const PAGE_FIELDS: [&str; 5] = ["id", "route", "state", "queries", "components"];
const QUERY_FIELDS: [&str; 2] = ["operation", "input"];

/// Parse the root uiko project source into located, authoring-neutral DTOs.
///
/// # Errors
///
/// Returns stable diagnostics when JSONC syntax or the root source shape is invalid.
pub fn parse_app_config(
    source_id: &SourceId,
    text: &str,
) -> Result<Located<AppConfigSource>, Vec<Diagnostic>> {
    let object = parse_root_object(source_id, text)?;
    let mut diagnostics = validate_properties(&object, &APP_FIELDS, "app", source_id);
    let name = parse_required_string(&object, "name", source_id, &mut diagnostics);
    let spec_version = parse_spec_version(&object, source_id, &mut diagnostics);
    let modules = parse_string_array(&object, "modules", source_id, &mut diagnostics);
    let integrations =
        parse_optional_string_array(&object, "integrations", source_id, &mut diagnostics);

    finish(
        diagnostics,
        (name, spec_version, modules, integrations),
        source_id,
        object.range,
        |(name, spec_version, modules, integrations)| {
            Some(AppConfigSource {
                name: name?,
                spec_version: spec_version?,
                modules: modules?,
                integrations: integrations?,
            })
        },
    )
}

/// Parse one module declaration.
///
/// # Errors
///
/// Returns stable diagnostics when JSONC syntax or the module shape is invalid.
pub fn parse_module_config(
    source_id: &SourceId,
    text: &str,
) -> Result<Located<ModuleConfigSource>, Vec<Diagnostic>> {
    let object = parse_root_object(source_id, text)?;
    let mut diagnostics = validate_properties(&object, &MODULE_FIELDS, "module", source_id);
    let id = parse_required_string(&object, "id", source_id, &mut diagnostics);
    let pages = parse_string_array(&object, "pages", source_id, &mut diagnostics);

    finish(
        diagnostics,
        (id, pages),
        source_id,
        object.range,
        |(id, pages)| {
            Some(ModuleConfigSource {
                id: id?,
                pages: pages?,
            })
        },
    )
}

/// Parse one capability integration declaration.
///
/// # Errors
///
/// Returns stable diagnostics when the integration source is invalid.
pub fn parse_integration_config(
    source_id: &SourceId,
    text: &str,
) -> Result<Located<IntegrationConfigSource>, Vec<Diagnostic>> {
    let object = parse_root_object(source_id, text)?;
    let mut diagnostics =
        validate_properties(&object, &INTEGRATION_FIELDS, "integration", source_id);
    let id = parse_required_string(&object, "id", source_id, &mut diagnostics);
    let adapter = parse_required_string(&object, "adapter", source_id, &mut diagnostics);
    let contract = parse_required_string(&object, "contract", source_id, &mut diagnostics);

    finish(
        diagnostics,
        (id, adapter, contract),
        source_id,
        object.range,
        |(id, adapter, contract)| {
            Some(IntegrationConfigSource {
                id: id?,
                adapter: adapter?,
                contract: contract?,
            })
        },
    )
}

/// Parse one page used by the G0 compiler slice.
///
/// # Errors
///
/// Returns stable diagnostics for malformed pages or unsupported component/query shapes.
pub fn parse_page(
    source_id: &SourceId,
    text: &str,
) -> Result<Located<PageSource>, Vec<Diagnostic>> {
    let object = parse_root_object(source_id, text)?;
    let mut diagnostics = validate_properties(&object, &PAGE_FIELDS, "page", source_id);
    let id = parse_required_string(&object, "id", source_id, &mut diagnostics);
    let route = parse_required_string(&object, "route", source_id, &mut diagnostics);
    let state = parse_state(&object, source_id, &mut diagnostics);
    let queries = parse_queries(&object, source_id, &mut diagnostics);
    let components = parse_components(&object, source_id, &mut diagnostics);

    finish(
        diagnostics,
        (id, route, state, queries, components),
        source_id,
        object.range,
        |(id, route, state, queries, components)| {
            Some(PageSource {
                id: id?,
                route: route?,
                state: state?,
                queries: queries?,
                components: components?,
            })
        },
    )
}

fn finish<T, I, F>(
    diagnostics: Vec<Diagnostic>,
    input: I,
    source_id: &SourceId,
    range: Range,
    build: F,
) -> Result<Located<T>, Vec<Diagnostic>>
where
    F: FnOnce(I) -> Option<T>,
{
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }
    build(input).map_or_else(
        || {
            Err(vec![Diagnostic::error(
                "UIKO1099",
                "source adapter could not construct a validated source model",
                span(source_id, range),
            )])
        },
        |value| Ok(Located::new(value, span(source_id, range))),
    )
}

fn parse_root_object<'a>(
    source_id: &SourceId,
    text: &'a str,
) -> Result<Object<'a>, Vec<Diagnostic>> {
    let parsed =
        parse_to_ast(text, &CollectOptions::default(), &parse_options()).map_err(|error| {
            vec![Diagnostic::error(
                "UIKO1000",
                error.kind().to_string(),
                span(source_id, error.range()),
            )]
        })?;

    let Some(value) = parsed.value else {
        return Err(vec![Diagnostic::error(
            "UIKO1002",
            "expected a root object",
            TextSpan::new(source_id.clone(), 0, 0),
        )]);
    };

    let range = value.range();
    match value {
        Value::Object(object) => Ok(object),
        _ => Err(vec![Diagnostic::error(
            "UIKO1002",
            "expected a root object",
            span(source_id, range),
        )]),
    }
}

fn parse_options() -> ParseOptions {
    ParseOptions {
        allow_comments: true,
        allow_loose_object_property_names: false,
        allow_trailing_commas: true,
        allow_missing_commas: false,
        allow_single_quoted_strings: false,
        allow_hexadecimal_numbers: false,
        allow_unary_plus_numbers: false,
    }
}

fn validate_properties(
    object: &Object<'_>,
    known_fields: &[&str],
    context: &str,
    source_id: &SourceId,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen = BTreeSet::new();

    for property in &object.properties {
        let name = property.name.as_str();
        if !known_fields.contains(&name) {
            diagnostics.push(Diagnostic::error(
                "UIKO1007",
                format!("unknown {context} property `{name}`"),
                span(source_id, property.name.range()),
            ));
        }
        if !seen.insert(name) {
            diagnostics.push(Diagnostic::error(
                "UIKO1006",
                format!("duplicate {context} property `{name}`"),
                span(source_id, property.name.range()),
            ));
        }
    }

    diagnostics
}

fn parse_required_string(
    object: &Object<'_>,
    field: &'static str,
    source_id: &SourceId,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Located<String>> {
    let property = required_property(object, field, source_id, diagnostics)?;
    string_value(property, field, source_id, diagnostics)
}

fn string_value(
    property: &ObjectProp<'_>,
    field: &str,
    source_id: &SourceId,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Located<String>> {
    match &property.value {
        Value::StringLit(value) => Some(Located::new(
            value.value.to_string(),
            span(source_id, value.range),
        )),
        value => {
            diagnostics.push(Diagnostic::error(
                "UIKO1004",
                format!("property `{field}` must be a string"),
                span(source_id, value.range()),
            ));
            None
        }
    }
}

fn parse_spec_version(
    object: &Object<'_>,
    source_id: &SourceId,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Located<u32>> {
    let property = required_property(object, "specVersion", source_id, diagnostics)?;
    let Value::NumberLit(value) = &property.value else {
        diagnostics.push(Diagnostic::error(
            "UIKO1004",
            "property `specVersion` must be a number",
            span(source_id, property.value.range()),
        ));
        return None;
    };

    if let Ok(version) = value.value.parse::<u32>() {
        Some(Located::new(version, span(source_id, value.range)))
    } else {
        diagnostics.push(Diagnostic::error(
            "UIKO1005",
            "property `specVersion` must be an unsigned integer",
            span(source_id, value.range),
        ));
        None
    }
}

fn parse_string_array(
    object: &Object<'_>,
    field: &'static str,
    source_id: &SourceId,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Vec<Located<String>>> {
    let property = required_property(object, field, source_id, diagnostics)?;
    string_array_value(property, field, source_id, diagnostics)
}

fn parse_optional_string_array(
    object: &Object<'_>,
    field: &'static str,
    source_id: &SourceId,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Vec<Located<String>>> {
    object.get(field).map_or(Some(Vec::new()), |property| {
        string_array_value(property, field, source_id, diagnostics)
    })
}

fn string_array_value(
    property: &ObjectProp<'_>,
    field: &str,
    source_id: &SourceId,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Vec<Located<String>>> {
    let Value::Array(array) = &property.value else {
        diagnostics.push(Diagnostic::error(
            "UIKO1004",
            format!("property `{field}` must be an array"),
            span(source_id, property.value.range()),
        ));
        return None;
    };

    let mut values = Vec::with_capacity(array.elements.len());
    for element in &array.elements {
        match element {
            Value::StringLit(value) => values.push(Located::new(
                value.value.to_string(),
                span(source_id, value.range),
            )),
            value => diagnostics.push(Diagnostic::error(
                "UIKO1008",
                format!("every `{field}` entry must be a string"),
                span(source_id, value.range()),
            )),
        }
    }
    Some(values)
}

fn parse_queries(
    object: &Object<'_>,
    source_id: &SourceId,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Vec<Located<QuerySource>>> {
    let Some(property) = object.get("queries") else {
        return Some(Vec::new());
    };
    let Value::Object(queries) = &property.value else {
        diagnostics.push(Diagnostic::error(
            "UIKO1004",
            "property `queries` must be an object",
            span(source_id, property.value.range()),
        ));
        return None;
    };

    let mut seen = BTreeSet::new();
    let mut result = Vec::with_capacity(queries.properties.len());

    for query_property in &queries.properties {
        let query_id = query_property.name.as_str();
        if !seen.insert(query_id) {
            diagnostics.push(Diagnostic::error(
                "UIKO1006",
                format!("duplicate query `{query_id}`"),
                span(source_id, query_property.name.range()),
            ));
            continue;
        }

        let Value::Object(query_object) = &query_property.value else {
            diagnostics.push(Diagnostic::error(
                "UIKO1012",
                format!("query `{query_id}` must be an object"),
                span(source_id, query_property.value.range()),
            ));
            continue;
        };

        diagnostics.extend(validate_properties(
            query_object,
            &QUERY_FIELDS,
            "query",
            source_id,
        ));

        let operation = parse_required_string(query_object, "operation", source_id, diagnostics);
        let input = parse_query_input(query_object, source_id, diagnostics);

        if let (Some(operation), Some(input)) = (operation, input) {
            result.push(Located::new(
                QuerySource {
                    id: Located::new(
                        query_id.to_string(),
                        span(source_id, query_property.name.range()),
                    ),
                    operation,
                    input,
                },
                span(source_id, query_property.value.range()),
            ));
        }
    }

    Some(result)
}

fn parse_query_input(
    query: &Object<'_>,
    source_id: &SourceId,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Vec<Located<InputBindingSource>>> {
    let property = required_property(query, "input", source_id, diagnostics)?;
    let Value::Object(input) = &property.value else {
        diagnostics.push(Diagnostic::error(
            "UIKO1013",
            "query `input` must be an object",
            span(source_id, property.value.range()),
        ));
        return None;
    };

    let mut seen = BTreeSet::new();
    let mut result = Vec::with_capacity(input.properties.len());

    for binding in &input.properties {
        let name = binding.name.as_str();
        if !seen.insert(name) {
            diagnostics.push(Diagnostic::error(
                "UIKO1006",
                format!("duplicate query input `{name}`"),
                span(source_id, binding.name.range()),
            ));
            continue;
        }

        let Some(expression) = string_value(binding, name, source_id, diagnostics) else {
            continue;
        };
        result.push(Located::new(
            InputBindingSource {
                name: Located::new(name.to_string(), span(source_id, binding.name.range())),
                expression,
            },
            span(source_id, binding.value.range()),
        ));
    }

    Some(result)
}

fn parse_state(
    object: &Object<'_>,
    source_id: &SourceId,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Vec<Located<StateSource>>> {
    let Some(property) = object.get("state") else {
        return Some(Vec::new());
    };
    let Value::Object(state) = &property.value else {
        diagnostics.push(Diagnostic::error(
            "UIKO1014",
            "property `state` must be an object",
            span(source_id, property.value.range()),
        ));
        return None;
    };

    let mut seen = BTreeSet::new();
    let mut result = Vec::with_capacity(state.properties.len());
    for entry in &state.properties {
        let id = entry.name.as_str();
        if !seen.insert(id) {
            diagnostics.push(Diagnostic::error(
                "UIKO1006",
                format!("duplicate state key `{id}`"),
                span(source_id, entry.name.range()),
            ));
            continue;
        }
        let Some(initial) = scalar_value(&entry.value, source_id, diagnostics) else {
            continue;
        };
        result.push(Located::new(
            StateSource {
                id: Located::new(id.to_string(), span(source_id, entry.name.range())),
                initial,
            },
            span(source_id, entry.value.range()),
        ));
    }
    Some(result)
}

fn scalar_value(
    value: &Value<'_>,
    source_id: &SourceId,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Located<ScalarValue>> {
    let scalar = match value {
        Value::StringLit(value) => ScalarValue::String(value.value.to_string()),
        Value::NumberLit(value) => ScalarValue::Number(value.value.to_string()),
        Value::BooleanLit(value) => ScalarValue::Boolean(value.value),
        Value::NullKeyword(_) => ScalarValue::Null,
        other => {
            diagnostics.push(Diagnostic::error(
                "UIKO1015",
                "state and option values must be JSON scalars",
                span(source_id, other.range()),
            ));
            return None;
        }
    };
    Some(Located::new(scalar, span(source_id, value.range())))
}

fn parse_optional_string(
    object: &Object<'_>,
    field: &'static str,
    source_id: &SourceId,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Option<String>> {
    object.get(field).map_or(Some(None), |property| {
        string_value(property, field, source_id, diagnostics).map(|value| Some(value.value))
    })
}

fn parse_table_columns(
    object: &Object<'_>,
    source_id: &SourceId,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Vec<TableColumnSource>> {
    let Some(property) = object.get("columns") else {
        return Some(Vec::new());
    };
    let Value::Array(columns) = &property.value else {
        diagnostics.push(Diagnostic::error(
            "UIKO1016",
            "Table `columns` must be an array",
            span(source_id, property.value.range()),
        ));
        return None;
    };

    let mut result = Vec::with_capacity(columns.elements.len());
    for value in &columns.elements {
        let Value::Object(column) = value else {
            diagnostics.push(Diagnostic::error(
                "UIKO1016",
                "each Table column must be an object",
                span(source_id, value.range()),
            ));
            continue;
        };
        diagnostics.extend(validate_properties(
            column,
            &["label", "binding"],
            "Table column",
            source_id,
        ));
        let label = parse_required_string(column, "label", source_id, diagnostics);
        let binding = parse_required_string(column, "binding", source_id, diagnostics);
        if let (Some(label), Some(binding)) = (label, binding) {
            result.push(TableColumnSource {
                label: label.value,
                binding: binding.value,
            });
        }
    }
    Some(result)
}

fn parse_select_options(
    object: &Object<'_>,
    source_id: &SourceId,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Vec<SelectOptionSource>> {
    let property = required_property(object, "options", source_id, diagnostics)?;
    let Value::Array(options) = &property.value else {
        diagnostics.push(Diagnostic::error(
            "UIKO1017",
            "Select `options` must be an array",
            span(source_id, property.value.range()),
        ));
        return None;
    };

    let mut result = Vec::with_capacity(options.elements.len());
    for value in &options.elements {
        let Value::Object(option) = value else {
            diagnostics.push(Diagnostic::error(
                "UIKO1017",
                "each Select option must be an object",
                span(source_id, value.range()),
            ));
            continue;
        };
        diagnostics.extend(validate_properties(
            option,
            &["label", "value"],
            "Select option",
            source_id,
        ));
        let label = parse_required_string(option, "label", source_id, diagnostics);
        let value_property = required_property(option, "value", source_id, diagnostics);
        let scalar = value_property.and_then(|property| {
            scalar_value(&property.value, source_id, diagnostics)
        });
        if let (Some(label), Some(value)) = (label, scalar) {
            result.push(SelectOptionSource {
                label: label.value,
                value: value.value,
            });
        }
    }
    Some(result)
}

fn parse_components(
    object: &Object<'_>,
    source_id: &SourceId,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Vec<Located<ComponentSource>>> {
    let property = required_property(object, "components", source_id, diagnostics)?;
    let Value::Array(array) = &property.value else {
        diagnostics.push(Diagnostic::error(
            "UIKO1004",
            "property `components` must be an array",
            span(source_id, property.value.range()),
        ));
        return None;
    };

    Some(
        array
            .elements
            .iter()
            .filter_map(|value| parse_component(value, source_id, diagnostics))
            .collect(),
    )
}

fn parse_component(
    value: &Value<'_>,
    source_id: &SourceId,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Located<ComponentSource>> {
    let Value::Object(object) = value else {
        diagnostics.push(Diagnostic::error(
            "UIKO1010",
            "component must be an object",
            span(source_id, value.range()),
        ));
        return None;
    };

    let id = parse_required_string(object, "id", source_id, diagnostics)?;
    let kind = parse_required_string(object, "type", source_id, diagnostics)?;
    let component_kind = match kind.value.as_str() {
        "Text" => {
            diagnostics.extend(validate_properties(
                object,
                &["id", "type", "value"],
                "Text component",
                source_id,
            ));
            ComponentKindSource::Text {
                value: parse_required_string(object, "value", source_id, diagnostics)?.value,
            }
        }
        "Field" => {
            diagnostics.extend(validate_properties(
                object,
                &["id", "type", "label", "binding", "fallback"],
                "Field component",
                source_id,
            ));
            ComponentKindSource::Field {
                label: parse_required_string(object, "label", source_id, diagnostics)?.value,
                binding: parse_required_string(object, "binding", source_id, diagnostics)?.value,
                fallback: parse_optional_string(object, "fallback", source_id, diagnostics)?,
            }
        }
        "Table" => {
            diagnostics.extend(validate_properties(
                object,
                &["id", "type", "binding", "columns"],
                "Table component",
                source_id,
            ));
            ComponentKindSource::Table {
                binding: parse_required_string(object, "binding", source_id, diagnostics)?.value,
                columns: parse_table_columns(object, source_id, diagnostics)?,
            }
        }
        "Select" => {
            diagnostics.extend(validate_properties(
                object,
                &["id", "type", "label", "state", "options"],
                "Select component",
                source_id,
            ));
            ComponentKindSource::Select {
                label: parse_required_string(object, "label", source_id, diagnostics)?.value,
                state: parse_required_string(object, "state", source_id, diagnostics)?.value,
                options: parse_select_options(object, source_id, diagnostics)?,
            }
        }
        "Pagination" => {
            diagnostics.extend(validate_properties(
                object,
                &[
                    "id",
                    "type",
                    "pageState",
                    "page",
                    "pageSize",
                    "total",
                ],
                "Pagination component",
                source_id,
            ));
            ComponentKindSource::Pagination {
                page_state: parse_required_string(
                    object,
                    "pageState",
                    source_id,
                    diagnostics,
                )?
                .value,
                page_binding: parse_required_string(object, "page", source_id, diagnostics)?.value,
                page_size_binding: parse_required_string(
                    object,
                    "pageSize",
                    source_id,
                    diagnostics,
                )?
                .value,
                total_binding: parse_required_string(object, "total", source_id, diagnostics)?.value,
            }
        }
        other => {
            diagnostics.push(Diagnostic::error(
                "UIKO1011",
                format!("unsupported component type `{other}`"),
                kind.span,
            ));
            return None;
        }
    };

    Some(Located::new(
        ComponentSource {
            id,
            kind: component_kind,
        },
        span(source_id, object.range),
    ))
}

fn required_property<'a>(
    object: &'a Object<'a>,
    field: &'static str,
    source_id: &SourceId,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<&'a ObjectProp<'a>> {
    let property = object.get(field);
    if property.is_none() {
        diagnostics.push(Diagnostic::error(
            "UIKO1003",
            format!("missing required property `{field}`"),
            span(source_id, object.range),
        ));
    }
    property
}

fn span(source_id: &SourceId, range: Range) -> TextSpan {
    TextSpan::new(source_id.clone(), range.start, range.end)
}

#[cfg(test)]
mod tests {
    use uiko_core::SourceId;

    use super::{parse_app_config, parse_integration_config, parse_module_config, parse_page};

    #[test]
    fn app_accepts_optional_integration_references() {
        let source = r#"{
  "name": "support-console",
  "specVersion": 1,
  "modules": ["./features/customers"],
  "integrations": ["./integrations/crm.jsonc"],
}"#;
        let app = parse_app_config(&SourceId::new("uiko.jsonc"), source).expect("valid app");
        assert_eq!(app.value.integrations.len(), 1);
    }

    #[test]
    fn app_policy_keeps_comments_and_trailing_commas_but_rejects_loose_json() {
        let valid = r#"{
  // project
  "name": "support-console",
  "specVersion": 1,
  "modules": ["./features/customers"],
}"#;
        assert!(parse_app_config(&SourceId::new("uiko.jsonc"), valid).is_ok());

        let invalid = r#"{ name: "support-console", "specVersion": 1, "modules": [] }"#;
        assert_eq!(
            parse_app_config(&SourceId::new("uiko.jsonc"), invalid).unwrap_err()[0].code,
            "UIKO1000"
        );
    }

    #[test]
    fn integration_shape_parses() {
        let source = r#"{
  "id": "crm",
  "adapter": "uiko.openapi-http",
  "contract": "./openapi/crm.json"
}"#;
        let integration =
            parse_integration_config(&SourceId::new("integrations/crm.jsonc"), source)
                .expect("valid integration");
        assert_eq!(integration.value.id.value, "crm");
        assert_eq!(integration.value.adapter.value, "uiko.openapi-http");
    }

    #[test]
    fn module_page_and_query_shapes_parse() {
        let module = r#"{
  "id": "customers",
  "pages": ["./detail.jsonc"]
}"#;
        let page = r#"{
  "id": "CustomerDetail",
  "route": "/customers/:customerId",
  "queries": {
    "customer": {
      "operation": "crm.getCustomer",
      "input": { "customerId": "route.customerId" }
    }
  },
  "components": [
    { "id": "name", "type": "Field", "label": "Name", "binding": "customer.name" }
  ]
}"#;

        let module = parse_module_config(&SourceId::new("features/customers/module.jsonc"), module)
            .expect("module should parse");
        let page = parse_page(&SourceId::new("features/customers/detail.jsonc"), page)
            .expect("page should parse");

        assert_eq!(module.value.id.value, "customers");
        assert_eq!(page.value.queries[0].value.id.value, "customer");
        assert_eq!(
            page.value.queries[0].value.input[0].value.expression.value,
            "route.customerId"
        );
    }

    #[test]
    fn unsupported_component_fails_closed() {
        let page = r#"{
  "id": "Broken",
  "route": "/broken",
  "components": [{ "id": "magic", "type": "Magic" }]
}"#;
        assert!(
            parse_page(&SourceId::new("broken.jsonc"), page)
                .unwrap_err()
                .iter()
                .any(|diagnostic| diagnostic.code == "UIKO1011")
        );
    }
}
