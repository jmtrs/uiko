#![forbid(unsafe_code)]

use std::fmt;

use uiko_capabilities::ValueShape;

/// Stable identifier for a source file known to the compiler.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SourceId(String);

impl SourceId {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Half-open byte range in one source file: `[start, end)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextSpan {
    pub source: SourceId,
    pub start: usize,
    pub end: usize,
}

impl TextSpan {
    /// # Panics
    ///
    /// Panics when `start > end`.
    #[must_use]
    pub fn new(source: SourceId, start: usize, end: usize) -> Self {
        assert!(start <= end, "span start must not exceed end");
        Self { source, start, end }
    }
}

/// A semantic value together with the source location that produced it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Located<T> {
    pub value: T,
    pub span: TextSpan,
}

impl<T> Located<T> {
    #[must_use]
    pub fn new(value: T, span: TextSpan) -> Self {
        Self { value, span }
    }
}

/// Stable diagnostic severity shared by source adapters and semantic passes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Severity {
    Error,
    Warning,
}

/// Machine-readable diagnostic emitted by uiko-owned compilation stages.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub code: &'static str,
    pub severity: Severity,
    pub message: String,
    pub span: TextSpan,
}

impl Diagnostic {
    #[must_use]
    pub fn error(code: &'static str, message: impl Into<String>, span: TextSpan) -> Self {
        Self {
            code,
            severity: Severity::Error,
            message: message.into(),
            span,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ModuleId(String);

impl ModuleId {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ModuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Private canonical semantic model. Renderer/framework concerns do not belong here.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppIr {
    pub app_name: String,
    pub spec_version: u32,
    pub modules: Vec<ModuleIr>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleIr {
    pub id: ModuleId,
    pub pages: Vec<PageIr>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageIr {
    pub id: String,
    pub route: String,
    pub queries: Vec<QueryIr>,
    pub components: Vec<ComponentIr>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueryIr {
    pub id: String,
    pub alias: String,
    pub provider_id: String,
    pub external_operation_id: String,
    pub input: Vec<QueryInputIr>,
    pub output: ValueShape,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueryInputIr {
    pub name: String,
    pub expression: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentIr {
    pub id: String,
    pub kind: ComponentKindIr,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ComponentKindIr {
    Text { value: String },
    Field { label: String, binding: String },
    Table { binding: String },
}
