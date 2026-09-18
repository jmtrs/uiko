#![forbid(unsafe_code)]

use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CapabilityCatalog {
    pub providers: BTreeMap<String, CapabilityProvider>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityProvider {
    pub id: String,
    pub operations: BTreeMap<String, QueryOperation>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueryOperation {
    pub external_id: String,
    pub parameters: Vec<OperationParameter>,
    pub output: ValueShape,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParameterLocation {
    Path,
    Query,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationParameter {
    pub name: String,
    pub location: ParameterLocation,
    pub required: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueShape {
    pub nullable: bool,
    pub kind: ValueKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValueKind {
    String,
    Integer,
    Number,
    Boolean,
    Array(Box<ValueShape>),
    Object(BTreeMap<String, ObjectField>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectField {
    pub required: bool,
    pub value: ValueShape,
}

impl ValueShape {
    #[must_use]
    pub fn supports_path<'a>(&self, path: impl IntoIterator<Item = &'a str>) -> bool {
        let mut current = self;
        for segment in path {
            match &current.kind {
                ValueKind::Object(fields) => {
                    let Some(field) = fields.get(segment) else {
                        return false;
                    };
                    current = &field.value;
                }
                ValueKind::Array(item) => {
                    current = item;
                    let ValueKind::Object(fields) = &current.kind else {
                        return false;
                    };
                    let Some(field) = fields.get(segment) else {
                        return false;
                    };
                    current = &field.value;
                }
                ValueKind::String | ValueKind::Integer | ValueKind::Number | ValueKind::Boolean => {
                    return false;
                }
            }
        }
        true
    }
}
