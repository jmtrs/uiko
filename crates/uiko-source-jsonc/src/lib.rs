#![forbid(unsafe_code)]

use std::collections::BTreeSet;

use jsonc_parser::{
    CollectOptions, ParseOptions,
    ast::{Object, ObjectProp, Value},
    common::{Range, Ranged},
    parse_to_ast,
};
use uiko_core::{Diagnostic, Located, SourceId, TextSpan};
use uiko_source::AppConfigSource;

const KNOWN_ROOT_FIELDS: [&str; 3] = ["name", "specVersion", "modules"];

/// Parse uiko's root JSONC project file into authoring-neutral located DTOs.
///
/// The parser library is intentionally configured explicitly because its
/// defaults accept syntax that uiko's source policy forbids.
///
/// # Errors
///
/// Returns stable uiko diagnostics for syntax and structural failures.
pub fn parse_app_config(
    source_id: SourceId,
    text: &str,
) -> Result<Located<AppConfigSource>, Vec<Diagnostic>> {
    let parse_result =
        parse_to_ast(text, &CollectOptions::default(), &parse_options()).map_err(|error| {
            vec![Diagnostic::error(
                "UIKO1000",
                error.kind().to_string(),
                span(&source_id, error.range()),
            )]
        })?;

    let Some(value) = parse_result.value else {
        return Err(vec![Diagnostic::error(
            "UIKO1002",
            "expected a root object",
            TextSpan::new(source_id, 0, 0),
        )]);
    };

    let root_range = value.range();
    let Value::Object(object) = value else {
        return Err(vec![Diagnostic::error(
            "UIKO1002",
            "expected a root object",
            span(&source_id, root_range),
        )]);
    };

    let mut diagnostics = validate_root_properties(&object, &source_id);

    let name = parse_required_string(&object, "name", &source_id, &mut diagnostics);
    let spec_version = parse_spec_version(&object, &source_id, &mut diagnostics);
    let modules = parse_modules(&object, &source_id, &mut diagnostics);

    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    match (name, spec_version, modules) {
        (Some(name), Some(spec_version), Some(modules)) => Ok(Located::new(
            AppConfigSource {
                name,
                spec_version,
                modules,
            },
            span(&source_id, object.range),
        )),
        _ => Err(vec![Diagnostic::error(
            "UIKO1099",
            "source adapter could not construct a validated root model",
            span(&source_id, object.range),
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

fn validate_root_properties(object: &Object<'_>, source_id: &SourceId) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen = BTreeSet::new();

    for property in &object.properties {
        let name = property.name.as_str();

        if !KNOWN_ROOT_FIELDS.contains(&name) {
            diagnostics.push(Diagnostic::error(
                "UIKO1007",
                format!("unknown root property `{name}`"),
                span(source_id, property.name.range()),
            ));
        }

        if !seen.insert(name) {
            diagnostics.push(Diagnostic::error(
                "UIKO1006",
                format!("duplicate root property `{name}`"),
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
                format!("root property `{field}` must be a string"),
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
        Value::NumberLit(value) => {
            if let Ok(version) = value.value.parse::<u32>() {
                Some(Located::new(version, span(source_id, value.range)))
            } else {
                diagnostics.push(Diagnostic::error(
                    "UIKO1005",
                    "root property `specVersion` must be an unsigned integer",
                    span(source_id, value.range),
                ));
                None
            }
        }
        value => {
            diagnostics.push(Diagnostic::error(
                "UIKO1004",
                "root property `specVersion` must be a number",
                span(source_id, value.range()),
            ));
            None
        }
    }
}

fn parse_modules(
    object: &Object<'_>,
    source_id: &SourceId,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Vec<Located<String>>> {
    let property = required_property(object, "modules", source_id, diagnostics)?;

    let Value::Array(array) = &property.value else {
        diagnostics.push(Diagnostic::error(
            "UIKO1004",
            "root property `modules` must be an array",
            span(source_id, property.value.range()),
        ));
        return None;
    };

    let mut modules = Vec::with_capacity(array.elements.len());
    for element in &array.elements {
        match element {
            Value::StringLit(value) => modules.push(Located::new(
                value.value.to_string(),
                span(source_id, value.range),
            )),
            value => diagnostics.push(Diagnostic::error(
                "UIKO1008",
                "every `modules` entry must be a string",
                span(source_id, value.range()),
            )),
        }
    }

    Some(modules)
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
            format!("missing required root property `{field}`"),
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

    use super::parse_app_config;

    fn parse(
        text: &str,
    ) -> Result<uiko_core::Located<uiko_source::AppConfigSource>, Vec<uiko_core::Diagnostic>> {
        parse_app_config(SourceId::new("uiko.jsonc"), text)
    }

    fn valid_root(extra: &str) -> String {
        format!(
            r#"{{
  "name": "support-console",
  "specVersion": 1,
  "modules": ["./features/customers"]{extra}
}}"#
        )
    }

    #[test]
    fn comments_and_trailing_commas_are_allowed() {
        let source = r#"{
  // project name
  "name": "support-console",
  "specVersion": 1,
  "modules": ["./features/customers"],
}"#;

        assert!(parse(source).is_ok());
    }

    #[test]
    fn loose_property_names_are_rejected() {
        let source = r#"{ name: "support-console", "specVersion": 1, "modules": [] }"#;
        assert_eq!(parse(source).unwrap_err()[0].code, "UIKO1000");
    }

    #[test]
    fn missing_commas_are_rejected() {
        let source = r#"{ "name": "support-console" "specVersion": 1, "modules": [] }"#;
        assert_eq!(parse(source).unwrap_err()[0].code, "UIKO1000");
    }

    #[test]
    fn single_quoted_strings_are_rejected() {
        let source = r#"{ "name": 'support-console', "specVersion": 1, "modules": [] }"#;
        assert_eq!(parse(source).unwrap_err()[0].code, "UIKO1000");
    }

    #[test]
    fn hexadecimal_numbers_are_rejected() {
        let source = r#"{ "name": "support-console", "specVersion": 0x1, "modules": [] }"#;
        assert_eq!(parse(source).unwrap_err()[0].code, "UIKO1000");
    }

    #[test]
    fn unary_plus_numbers_are_rejected() {
        let source = r#"{ "name": "support-console", "specVersion": +1, "modules": [] }"#;
        assert_eq!(parse(source).unwrap_err()[0].code, "UIKO1000");
    }

    #[test]
    fn located_values_keep_exact_byte_ranges() {
        let source = valid_root("");
        let parsed = parse(&source).expect("valid root should parse");

        let name_span = parsed.value.name.span;
        assert_eq!(
            &source[name_span.start..name_span.end],
            r#""support-console""#
        );

        let module_span = &parsed.value.modules[0].span;
        assert_eq!(
            &source[module_span.start..module_span.end],
            r#""./features/customers""#
        );
    }

    #[test]
    fn unknown_root_properties_fail_closed() {
        let source = valid_root(r#", "surprise": true"#);
        let diagnostics = parse(&source).expect_err("unknown root property must fail");

        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "UIKO1007")
        );
    }

    #[test]
    fn duplicate_root_properties_fail_closed() {
        let source = r#"{
  "name": "support-console",
  "name": "other",
  "specVersion": 1,
  "modules": []
}"#;
        let diagnostics = parse(source).expect_err("duplicate root property must fail");

        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "UIKO1006")
        );
    }

    #[test]
    fn malformed_spec_version_has_its_own_structural_diagnostic() {
        let source = r#"{ "name": "support-console", "specVersion": 1.0, "modules": [] }"#;
        let diagnostics = parse(source).expect_err("non-integer specVersion must fail");

        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "UIKO1005")
        );
    }

    #[test]
    fn non_string_module_entry_is_rejected() {
        let source = r#"{ "name": "support-console", "specVersion": 1, "modules": [42] }"#;
        let diagnostics = parse(source).expect_err("non-string module entry must fail");

        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "UIKO1008")
        );
    }
}
