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

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
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
    pub fn at_path<'a, 'b>(
        &'a self,
        path: impl IntoIterator<Item = &'b str>,
    ) -> Option<&'a ValueShape> {
        let mut current = self;
        for segment in path {
            match &current.kind {
                ValueKind::Object(fields) => {
                    current = &fields.get(segment)?.value;
                }
                ValueKind::Array(item) => {
                    let ValueKind::Object(fields) = &item.kind else {
                        return None;
                    };
                    current = &fields.get(segment)?.value;
                }
                ValueKind::String | ValueKind::Integer | ValueKind::Number | ValueKind::Boolean => {
                    return None;
                }
            }
        }
        Some(current)
    }

    #[must_use]
    pub fn supports_path<'a>(&self, path: impl IntoIterator<Item = &'a str>) -> bool {
        self.at_path(path).is_some()
    }

    #[must_use]
    pub fn array_item(&self) -> Option<&ValueShape> {
        match &self.kind {
            ValueKind::Array(item) => Some(item),
            ValueKind::String
            | ValueKind::Integer
            | ValueKind::Number
            | ValueKind::Boolean
            | ValueKind::Object(_) => None,
        }
    }

    #[must_use]
    pub const fn is_numeric(&self) -> bool {
        matches!(self.kind, ValueKind::Integer | ValueKind::Number)
    }
}
