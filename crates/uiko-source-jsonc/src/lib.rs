#![forbid(unsafe_code)]

use std::collections::BTreeSet;

use jsonc_parser::{
    CollectOptions, ParseOptions,
    ast::{Object, ObjectProp, Value},
    common::{Range, Ranged},
    parse_to_ast,
};
use uiko_core::{Diagnostic, Located, SourceId, TextSpan};
use uiko_source::{AppConfigSource, ComponentSource, ModuleConfigSource, PageSource};

const APP_FIELDS: [&str; 3] = ["name", "specVersion", "modules"];
const MODULE_FIELDS: [&str; 2] = ["id", "pages"];
const PAGE_FIELDS: [&str; 3] = ["id", "route", "components"];

pub fn parse_app_config(
    source_id: SourceId,
    text: &str,
) -> Result<Located<AppConfigSource>, Vec<Diagnostic>> {
    let object = parse_root_object(&source_id, text)?;
    let mut diagnostics = validate_properties(&object, &APP_FIELDS, "app", &source_id);
    let name = parse_required_string(&object, "name", &source_id, &mut diagnostics);
    let spec_version = parse_spec_version(&object, &source_id, &mut diagnostics);
    let modules = parse_string_array(&object, "modules", &source_id, &mut diagnostics);
    finish(
        diagnostics,
        (name, spec_version, modules),
        &source_id,
        object.range,
        |(name, spec_version, modules)| AppConfigSource {
            name: name?,
            spec_version: spec_version?,
            modules: modules?,
        },
    )
}

pub fn parse_module_config(
    source_id: SourceId,
    text: &str,
) -> Result<Located<ModuleConfigSource>, Vec<Diagnostic>> {
    let object = parse_root_object(&source_id, text)?;
    let mut diagnostics = validate_properties(&object, &MODULE_FIELDS, "module", &source_id);
    let id = parse_required_string(&object, "id", &source_id, &mut diagnostics);
    let pages = parse_string_array(&object, "pages", &source_id, &mut diagnostics);
    finish(
        diagnostics,
        (id, pages),
        &source_id,
        object.range,
        |(id, pages)| ModuleConfigSource {
            id: id?,
            pages: pages?,
        },
    )
}

pub fn parse_page(
    source_id: SourceId,
    text: &str,
) -> Result<Located<PageSource>, Vec<Diagnostic>> {
    let object = parse_root_object(&source_id, text)?;
    let mut diagnostics = validate_properties(&object, &PAGE_FIELDS, "page", &source_id);
    let id = parse_required_string(&object, "id", &source_id, &mut diagnostics);
    let route = parse_required_string(&object, "route", &source_id, &mut diagnostics);
    let components = parse_components(&object, &source_id, &mut diagnostics);
    finish(
        diagnostics,
        (id, route, components),
        &source_id,
        object.range,
        |(id, route, components)| PageSource {
            id: id?,
            route: route?,
            components: components?,
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
    match &property.value {
        Value::NumberLit(value) => match value.value.parse::<u32>() {
            Ok(version) => Some(Located::new(version, span(source_id, value.range))),
            Err(_) => {
                diagnostics.push(Diagnostic::error(
                    "UIKO1005",
                    "property `specVersion` must be an unsigned integer",
                    span(source_id, value.range),
                ));
                None
            }
        },
        value => {
            diagnostics.push(Diagnostic::error(
                "UIKO1004",
                "property `specVersion` must be a number",
                span(source_id, value.range()),
            ));
            None
        }
    }
}

fn parse_string_array(
    object: &Object<'_>,
    field: &'static str,
    source_id: &SourceId,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Vec<Located<String>>> {
    let property = required_property(object, field, source_id, diagnostics)?;
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

    let kind = parse_required_string(object, "type", source_id, diagnostics)?;
    let component = match kind.value.as_str() {
        "Text" => {
            diagnostics.extend(validate_properties(
                object,
                &["type", "value"],
                "Text component",
                source_id,
            ));
            ComponentSource::Text {
                value: parse_required_string(object, "value", source_id, diagnostics)?.value,
            }
        }
        "Field" => {
            diagnostics.extend(validate_properties(
                object,
                &["type", "label", "binding"],
                "Field component",
                source_id,
            ));
            ComponentSource::Field {
                label: parse_required_string(object, "label", source_id, diagnostics)?.value,
                binding: parse_required_string(object, "binding", source_id, diagnostics)?.value,
            }
        }
        "Table" => {
            diagnostics.extend(validate_properties(
                object,
                &["type", "binding"],
                "Table component",
                source_id,
            ));
            ComponentSource::Table {
                binding: parse_required_string(object, "binding", source_id, diagnostics)?.value,
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

    Some(Located::new(component, span(source_id, object.range)))
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

    use super::{parse_app_config, parse_module_config, parse_page};

    #[test]
    fn app_policy_keeps_comments_and_trailing_commas_but_rejects_loose_json() {
        let valid = r#"{
  // project
  "name": "support-console",
  "specVersion": 1,
  "modules": ["./features/customers"],
}"#;
        assert!(parse_app_config(SourceId::new("uiko.jsonc"), valid).is_ok());

        let invalid = r#"{ name: "support-console", "specVersion": 1, "modules": [] }"#;
        assert_eq!(
            parse_app_config(SourceId::new("uiko.jsonc"), invalid).unwrap_err()[0].code,
            "UIKO1000"
        );
    }

    #[test]
    fn located_values_keep_exact_byte_ranges() {
        let source = r#"{
  "name": "support-console",
  "specVersion": 1,
  "modules": ["./features/customers"]
}"#;
        let parsed =
            parse_app_config(SourceId::new("uiko.jsonc"), source).expect("valid app config");

        let name = &parsed.value.name.span;
        assert_eq!(&source[name.start..name.end], r#""support-console""#);
        let module = &parsed.value.modules[0].span;
        assert_eq!(&source[module.start..module.end], r#""./features/customers""#);
    }

    #[test]
    fn unknown_and_duplicate_properties_fail_closed() {
        let unknown =
            r#"{ "name": "x", "specVersion": 1, "modules": [], "surprise": true }"#;
        assert!(
            parse_app_config(SourceId::new("uiko.jsonc"), unknown)
                .unwrap_err()
                .iter()
                .any(|diagnostic| diagnostic.code == "UIKO1007")
        );

        let duplicate =
            r#"{ "name": "x", "name": "y", "specVersion": 1, "modules": [] }"#;
        assert!(
            parse_app_config(SourceId::new("uiko.jsonc"), duplicate)
                .unwrap_err()
                .iter()
                .any(|diagnostic| diagnostic.code == "UIKO1006")
        );
    }

    #[test]
    fn module_and_page_fixture_shapes_parse() {
        let module = r#"{
  "id": "customers",
  "pages": ["./list.jsonc", "./detail.jsonc"]
}"#;
        let page = r#"{
  "id": "CustomerDetail",
  "route": "/customers/:customerId",
  "components": [
    { "type": "Text", "value": "Customer" },
    { "type": "Field", "label": "Name", "binding": "customer.name" }
  ]
}"#;

        let module = parse_module_config(SourceId::new("features/customers/module.jsonc"), module)
            .expect("module should parse");
        let page = parse_page(SourceId::new("features/customers/detail.jsonc"), page)
            .expect("page should parse");

        assert_eq!(module.value.id.value, "customers");
        assert_eq!(module.value.pages.len(), 2);
        assert_eq!(page.value.id.value, "CustomerDetail");
        assert_eq!(page.value.components.len(), 2);
    }

    #[test]
    fn unsupported_component_fails_closed() {
        let page = r#"{
  "id": "Broken",
  "route": "/broken",
  "components": [{ "type": "Magic" }]
}"#;
        assert!(
            parse_page(SourceId::new("broken.jsonc"), page)
                .unwrap_err()
                .iter()
                .any(|diagnostic| diagnostic.code == "UIKO1011")
        );
    }
}
