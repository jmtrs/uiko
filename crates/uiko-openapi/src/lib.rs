#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use oas3::spec::Spec;
use serde_json::{Map, Value};
use uiko_capabilities::{
    CapabilityProvider, ObjectField, OperationParameter, ParameterLocation, QueryOperation,
    ValueKind, ValueShape,
};
use uiko_core::{Diagnostic, SourceId, TextSpan};

const ADAPTER_ID: &str = "uiko.openapi-http";

/// Import the supported OpenAPI 3.1 read-only subset into uiko-owned capability semantics.
///
/// Parsing is delegated to `oas3`; this strict visitor then accepts only the
/// subset uiko knows how to reason about.
///
/// # Errors
///
/// Returns stable diagnostics for invalid OpenAPI documents or unsupported
/// constructs.
pub fn import_openapi_provider(
    provider_id: &str,
    source_id: &SourceId,
    text: &str,
) -> Result<CapabilityProvider, Vec<Diagnostic>> {
    let raw: Value = serde_json::from_str(text).map_err(|error| {
        vec![diagnostic(
            "UIKO2000",
            format!("invalid OpenAPI JSON: {error}"),
            source_id,
            text,
        )]
    })?;

    let spec: Spec = serde_json::from_value(raw.clone()).map_err(|error| {
        vec![diagnostic(
            "UIKO2000",
            format!("invalid OpenAPI document: {error}"),
            source_id,
            text,
        )]
    })?;

    if !spec.openapi.starts_with("3.1.") {
        return Err(vec![diagnostic(
            "UIKO2001",
            format!(
                "unsupported OpenAPI version `{}`; M4 requires OpenAPI 3.1.x",
                spec.openapi
            ),
            source_id,
            text,
        )]);
    }

    if raw.get("jsonSchemaDialect").is_some() {
        return Err(vec![diagnostic(
            "UIKO2002",
            "custom jsonSchemaDialect is not supported by the G0 OpenAPI subset",
            source_id,
            text,
        )]);
    }

    let paths = raw
        .get("paths")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            vec![diagnostic(
                "UIKO2003",
                "OpenAPI document must contain a paths object",
                source_id,
                text,
            )]
        })?;

    let mut operations = BTreeMap::new();
    let mut diagnostics = Vec::new();

    for (path, method, operation) in spec.operations() {
        if !method.as_str().eq_ignore_ascii_case("GET") {
            continue;
        }

        let Some(operation_id) = operation.operation_id.as_deref() else {
            diagnostics.push(diagnostic(
                "UIKO2004",
                format!("GET operation `{path}` is missing operationId"),
                source_id,
                text,
            ));
            continue;
        };

        match import_get_operation(&raw, paths, &path, method.as_str(), operation_id) {
            Ok(normalized) => {
                operations.insert(operation_id.to_string(), normalized);
            }
            Err(message) => diagnostics.push(diagnostic(
                "UIKO2005",
                format!("operation `{operation_id}`: {message}"),
                source_id,
                text,
            )),
        }
    }

    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    Ok(CapabilityProvider {
        id: provider_id.to_string(),
        operations,
    })
}

#[must_use]
pub const fn adapter_id() -> &'static str {
    ADAPTER_ID
}

fn import_get_operation(
    raw: &Value,
    paths: &Map<String, Value>,
    path: &str,
    method: &str,
    operation_id: &str,
) -> Result<QueryOperation, String> {
    let operation = paths
        .get(path)
        .and_then(Value::as_object)
        .and_then(|path_item| path_item.get(&method.to_ascii_lowercase()))
        .and_then(Value::as_object)
        .ok_or_else(|| "typed parser operation could not be located in source JSON".to_string())?;

    let parameters = import_parameters(operation)?;
    let output = import_success_output(raw, operation)?;

    Ok(QueryOperation {
        external_id: operation_id.to_string(),
        parameters,
        output,
    })
}

fn import_parameters(operation: &Map<String, Value>) -> Result<Vec<OperationParameter>, String> {
    let Some(parameters) = operation.get("parameters") else {
        return Ok(Vec::new());
    };
    let parameters = parameters
        .as_array()
        .ok_or_else(|| "parameters must be an array".to_string())?;

    let mut result = Vec::with_capacity(parameters.len());
    let mut seen = BTreeSet::new();

    for parameter in parameters {
        let parameter = parameter
            .as_object()
            .ok_or_else(|| "parameter references are not supported in M4".to_string())?;
        if parameter.contains_key("$ref") {
            return Err("parameter $ref is not supported in M4".to_string());
        }

        let name = parameter
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| "parameter must have a string name".to_string())?;
        let location = match parameter.get("in").and_then(Value::as_str) {
            Some("path") => ParameterLocation::Path,
            Some("query") => ParameterLocation::Query,
            Some(other) => {
                return Err(format!(
                    "parameter `{name}` uses unsupported location `{other}`"
                ));
            }
            None => return Err(format!("parameter `{name}` is missing `in`")),
        };
        if !seen.insert((name.to_string(), location)) {
            return Err(format!("duplicate parameter `{name}`"));
        }

        if parameter.get("content").is_some() {
            return Err(format!(
                "parameter `{name}` content encoding is not supported in M4"
            ));
        }
        if parameter.get("schema").is_none() {
            return Err(format!("parameter `{name}` must declare schema"));
        }

        let required = parameter
            .get("required")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        if location == ParameterLocation::Path && !required {
            return Err(format!("path parameter `{name}` must be required"));
        }

        result.push(OperationParameter {
            name: name.to_string(),
            location,
            required,
        });
    }

    Ok(result)
}

fn import_success_output(raw: &Value, operation: &Map<String, Value>) -> Result<ValueShape, String> {
    let responses = operation
        .get("responses")
        .and_then(Value::as_object)
        .ok_or_else(|| "responses must be an object".to_string())?;

    let response = responses
        .get("200")
        .or_else(|| {
            responses
                .iter()
                .find(|(status, _)| status.starts_with('2'))
                .map(|(_, response)| response)
        })
        .ok_or_else(|| "a JSON success response is required".to_string())?;

    let response = response
        .as_object()
        .ok_or_else(|| "response references are not supported in M4".to_string())?;
    if response.contains_key("$ref") {
        return Err("response $ref is not supported in M4".to_string());
    }

    let schema = response
        .get("content")
        .and_then(Value::as_object)
        .and_then(|content| content.get("application/json"))
        .and_then(Value::as_object)
        .and_then(|media| media.get("schema"))
        .ok_or_else(|| "success response must declare application/json schema".to_string())?;

    let mut stack = BTreeSet::new();
    import_schema(raw, schema, &mut stack)
}

fn import_schema(
    raw: &Value,
    schema: &Value,
    stack: &mut BTreeSet<String>,
) -> Result<ValueShape, String> {
    let object = schema
        .as_object()
        .ok_or_else(|| "schema must be an object".to_string())?;

    if let Some(reference) = object.get("$ref").and_then(Value::as_str) {
        let name = reference
            .strip_prefix("#/components/schemas/")
            .ok_or_else(|| format!("only local component schema refs are supported: {reference}"))?;
        if !stack.insert(name.to_string()) {
            return Err(format!("recursive schema `{name}` is not supported in M4"));
        }

        let resolved = raw
            .pointer(&format!("/components/schemas/{name}"))
            .ok_or_else(|| format!("schema reference `{reference}` cannot be resolved"))?;
        let result = import_schema(raw, resolved, stack);
        stack.remove(name);
        return result;
    }

    reject_unsupported_schema_keywords(object)?;

    let (type_name, nullable) = parse_type(object)?;
    let kind = match type_name {
        "string" => ValueKind::String,
        "integer" => ValueKind::Integer,
        "number" => ValueKind::Number,
        "boolean" => ValueKind::Boolean,
        "array" => {
            let items = object
                .get("items")
                .ok_or_else(|| "array schema must declare items".to_string())?;
            ValueKind::Array(Box::new(import_schema(raw, items, stack)?))
        }
        "object" => {
            let properties = object
                .get("properties")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            let required: BTreeSet<&str> = object
                .get("required")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .collect();

            let mut fields = BTreeMap::new();
            for (name, field_schema) in properties {
                fields.insert(
                    name.clone(),
                    ObjectField {
                        required: required.contains(name.as_str()),
                        value: import_schema(raw, &field_schema, stack)?,
                    },
                );
            }
            ValueKind::Object(fields)
        }
        other => return Err(format!("unsupported schema type `{other}`")),
    };

    Ok(ValueShape { nullable, kind })
}

fn parse_type(object: &Map<String, Value>) -> Result<(&str, bool), String> {
    match object.get("type") {
        Some(Value::String(value)) => Ok((value.as_str(), false)),
        Some(Value::Array(values)) => {
            let mut non_null = values
                .iter()
                .filter_map(Value::as_str)
                .filter(|value| *value != "null");
            let Some(value) = non_null.next() else {
                return Err("schema type cannot contain only null".to_string());
            };
            if non_null.next().is_some()
                || !values
                    .iter()
                    .filter_map(Value::as_str)
                    .any(|value| value == "null")
            {
                return Err(
                    "M4 supports only one concrete schema type plus optional null".to_string(),
                );
            }
            Ok((value, true))
        }
        Some(_) => Err("schema type must be a string or string array".to_string()),
        None => Err("schema type is required in the M4 subset".to_string()),
    }
}

fn reject_unsupported_schema_keywords(object: &Map<String, Value>) -> Result<(), String> {
    const ALLOWED: &[&str] = &[
        "type",
        "properties",
        "required",
        "items",
        "format",
        "pattern",
        "minLength",
        "maxLength",
        "minimum",
        "maximum",
        "default",
        "enum",
        "description",
        "title",
        "readOnly",
        "writeOnly",
        "examples",
    ];

    for key in object.keys() {
        if key == "$ref" || ALLOWED.contains(&key.as_str()) {
            continue;
        }
        return Err(format!("unsupported schema keyword `{key}`"));
    }

    Ok(())
}

fn diagnostic(code: &'static str, message: impl Into<String>, source_id: &SourceId, text: &str) -> Diagnostic {
    Diagnostic::error(
        code,
        message,
        TextSpan::new(source_id.clone(), 0, text.len()),
    )
}

#[cfg(test)]
mod tests {
    use uiko_capabilities::{ParameterLocation, ValueKind};
    use uiko_core::SourceId;

    use super::import_openapi_provider;

    const CONTRACT: &str = r##"{
      "openapi":"3.1.0",
      "info":{"title":"test","version":"1"},
      "paths":{
        "/customers/{customerId}":{
          "get":{
            "operationId":"getCustomer",
            "parameters":[
              {"name":"customerId","in":"path","required":true,"schema":{"type":"string"}}
            ],
            "responses":{
              "200":{
                "description":"ok",
                "content":{
                  "application/json":{
                    "schema":{"$ref":"#/components/schemas/Customer"}
                  }
                }
              }
            }
          }
        }
      },
      "components":{
        "schemas":{
          "Customer":{
            "type":"object",
            "required":["id","name"],
            "properties":{
              "id":{"type":"string"},
              "name":{"type":"string"},
              "phone":{"type":["string","null"]}
            }
          }
        }
      }
    }"##;

    #[test]
    fn imports_get_operation_into_protocol_neutral_shape() {
        let provider =
            import_openapi_provider("crm", &SourceId::new("crm.json"), CONTRACT).expect("contract");

        let operation = provider.operations.get("getCustomer").expect("operation");
        assert_eq!(operation.parameters[0].location, ParameterLocation::Path);
        assert!(operation.parameters[0].required);
        assert!(matches!(operation.output.kind, ValueKind::Object(_)));
        assert!(operation.output.supports_path(["name"]));
        assert!(operation.output.supports_path(["phone"]));
    }

    #[test]
    fn unsupported_composition_fails_closed() {
        let contract = CONTRACT.replace(
            r#""name":{"type":"string"}"#,
            r#""name":{"type":"string","oneOf":[{"type":"string"}]}"#,
        );
        let diagnostics =
            import_openapi_provider("crm", &SourceId::new("crm.json"), &contract).unwrap_err();

        assert_eq!(diagnostics[0].code, "UIKO2005");
        assert!(diagnostics[0].message.contains("oneOf"));
    }
}
