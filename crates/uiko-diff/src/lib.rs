#![forbid(unsafe_code)]

//! Compile-time semantic diff over two compiled uiko plans (M13, spec §16.3).
//!
//! Every semantic change between two `AppIr` revisions is represented as a
//! `DiffRow`; the report is deterministic (byte-identical for identical
//! inputs) and marks security-significant rows so review tooling can gate on
//! them. Capability scoping is app-referenced only: unreferenced catalog
//! churn produces no row.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fmt::Write as _;
use std::mem;

use uiko_capabilities::{CapabilityCatalog, OperationParameter, ValueKind, ValueShape};
use uiko_core::{AppIr, ComponentIr, ComponentKindIr, ExecutionModeIr, StateValueIr};

/// Hand-rolled canonical JSON value for deterministic diff payloads.
///
/// Object keys are static by construction and keep insertion order; every
/// collection fed into payloads is sorted before construction.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Str(String),
    Int(i64),
    Arr(Vec<JsonValue>),
    Obj(Vec<(&'static str, JsonValue)>),
}

impl fmt::Display for JsonValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JsonValue::Null => f.write_str("null"),
            JsonValue::Bool(value) => write!(f, "{value}"),
            JsonValue::Str(value) => write!(f, "\"{}\"", json_escape(value)),
            JsonValue::Int(value) => write!(f, "{value}"),
            JsonValue::Arr(elements) => {
                f.write_str("[")?;
                for (index, element) in elements.iter().enumerate() {
                    if index > 0 {
                        f.write_str(",")?;
                    }
                    write!(f, "{element}")?;
                }
                f.write_str("]")
            }
            JsonValue::Obj(properties) => {
                f.write_str("{")?;
                for (index, (key, value)) in properties.iter().enumerate() {
                    if index > 0 {
                        f.write_str(",")?;
                    }
                    write!(f, "\"{key}\":{value}")?;
                }
                f.write_str("}")
            }
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

/// One dimension of the diff taxonomy. The string form is the stable row kind.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DiffDimension {
    AppNameChanged,
    AppSpecVersionChanged,
    ModuleAdded,
    ModuleRemoved,
    PageAdded,
    PageRemoved,
    PageRouteChanged,
    StateAdded,
    StateRemoved,
    StateInitialChanged,
    QueryAdded,
    QueryRemoved,
    QueryOperationRemapped,
    QueryInputAdded,
    QueryInputRemoved,
    QueryInputRebound,
    QueryOutputChanged,
    CapabilityParametersChanged,
    QueryExecutionChanged,
    QueryAuthorizationChanged,
    ComponentAdded,
    ComponentRemoved,
    ComponentKindChanged,
    ComponentFieldChanged,
    ComponentOptionsChanged,
    ComponentReordered,
}

impl DiffDimension {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            DiffDimension::AppNameChanged => "app.name.changed",
            DiffDimension::AppSpecVersionChanged => "app.spec_version.changed",
            DiffDimension::ModuleAdded => "module.added",
            DiffDimension::ModuleRemoved => "module.removed",
            DiffDimension::PageAdded => "page.added",
            DiffDimension::PageRemoved => "page.removed",
            DiffDimension::PageRouteChanged => "page.route.changed",
            DiffDimension::StateAdded => "state.added",
            DiffDimension::StateRemoved => "state.removed",
            DiffDimension::StateInitialChanged => "state.initial.changed",
            DiffDimension::QueryAdded => "query.added",
            DiffDimension::QueryRemoved => "query.removed",
            DiffDimension::QueryOperationRemapped => "query.operation.remapped",
            DiffDimension::QueryInputAdded => "query.input.added",
            DiffDimension::QueryInputRemoved => "query.input.removed",
            DiffDimension::QueryInputRebound => "query.input.rebound",
            DiffDimension::QueryOutputChanged => "query.output.changed",
            DiffDimension::CapabilityParametersChanged => "capability.parameters.changed",
            DiffDimension::QueryExecutionChanged => "query.execution.changed",
            DiffDimension::QueryAuthorizationChanged => "query.authorization.changed",
            DiffDimension::ComponentAdded => "component.added",
            DiffDimension::ComponentRemoved => "component.removed",
            DiffDimension::ComponentKindChanged => "component.kind.changed",
            DiffDimension::ComponentFieldChanged => "component.field.changed",
            DiffDimension::ComponentOptionsChanged => "component.options.changed",
            DiffDimension::ComponentReordered => "component.reordered",
        }
    }
}

/// One semantic change between two compiled plans.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiffRow {
    pub kind: String,
    pub target: String,
    pub before: Option<JsonValue>,
    pub after: Option<JsonValue>,
    pub security_significant: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiffStatus {
    Clean,
    Changed,
}

impl DiffStatus {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            DiffStatus::Clean => "clean",
            DiffStatus::Changed => "changed",
        }
    }
}

/// Deterministic semantic diff between two compiled plans.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiffReport {
    pub status: DiffStatus,
    pub rows: Vec<DiffRow>,
}

impl DiffReport {
    /// Canonical compact JSON envelope: `{"status":...,"rows":[...]}`.
    #[must_use]
    pub fn to_json(&self) -> String {
        let rows = self
            .rows
            .iter()
            .map(|row| {
                let mut properties = vec![
                    ("kind", JsonValue::Str(row.kind.clone())),
                    ("target", JsonValue::Str(row.target.clone())),
                    (
                        "securitySignificant",
                        JsonValue::Bool(row.security_significant),
                    ),
                ];
                if let Some(before) = &row.before {
                    properties.push(("before", before.clone()));
                }
                if let Some(after) = &row.after {
                    properties.push(("after", after.clone()));
                }
                JsonValue::Obj(properties)
            })
            .collect();

        JsonValue::Obj(vec![
            ("status", JsonValue::Str(self.status.as_str().into())),
            ("rows", JsonValue::Arr(rows)),
        ])
        .to_string()
    }

    /// Human-readable rendering: header plus one `UIKO-DIFF` line per row.
    #[must_use]
    pub fn to_human(&self) -> String {
        let mut output = String::new();
        if self.rows.is_empty() {
            output.push_str("uiko diff: clean\n");
            return output;
        }

        writeln!(output, "uiko diff: changed ({} rows)", self.rows.len())
            .expect("writing to String cannot fail");
        for row in &self.rows {
            let marker = if row.security_significant {
                " security-significant"
            } else {
                ""
            };
            writeln!(output, "UIKO-DIFF {} {}{}", row.kind, row.target, marker)
                .expect("writing to String cannot fail");
            match (&row.before, &row.after) {
                // Both sides present and at least one is structured: print a
                // field-level delta instead of two opaque single-line blobs, so
                // a human reviewer sees only what changed. The JSON envelope
                // still carries the full before/after for machines.
                (Some(before), Some(after)) if is_structured(before) || is_structured(after) => {
                    let mut lines = Vec::new();
                    structural_delta(before, after, String::new(), &mut lines);
                    if lines.is_empty() {
                        writeln!(output, "  before: {before}")
                            .expect("writing to String cannot fail");
                        writeln!(output, "  after: {after}")
                            .expect("writing to String cannot fail");
                    } else {
                        output.push_str("  changes:\n");
                        for line in lines {
                            writeln!(output, "    {line}").expect("writing to String cannot fail");
                        }
                    }
                }
                _ => {
                    if let Some(before) = &row.before {
                        writeln!(output, "  before: {before}")
                            .expect("writing to String cannot fail");
                    }
                    if let Some(after) = &row.after {
                        writeln!(output, "  after: {after}")
                            .expect("writing to String cannot fail");
                    }
                }
            }
        }
        output
    }
}

/// True for values whose single-line rendering is unreadable at size: objects
/// and arrays. Scalars stay on the plain `before:`/`after:` lines.
fn is_structured(value: &JsonValue) -> bool {
    matches!(value, JsonValue::Obj(_) | JsonValue::Arr(_))
}

/// Path-addressed structural delta between two payloads. Emits one line per
/// changed leaf: `~ path: before -> after`, `+ path: added`, `- path: removed`.
/// Deterministic: object keys walked in sorted order, arrays by index.
fn structural_delta(before: &JsonValue, after: &JsonValue, path: String, out: &mut Vec<String>) {
    if before == after {
        return;
    }
    match (before, after) {
        (JsonValue::Obj(a), JsonValue::Obj(b)) => {
            let a_map: BTreeMap<&str, &JsonValue> = a.iter().map(|(k, v)| (*k, v)).collect();
            let b_map: BTreeMap<&str, &JsonValue> = b.iter().map(|(k, v)| (*k, v)).collect();
            let keys: BTreeSet<&str> = a_map.keys().chain(b_map.keys()).copied().collect();
            for key in keys {
                let child = if path.is_empty() {
                    key.to_string()
                } else {
                    format!("{path}.{key}")
                };
                match (a_map.get(key), b_map.get(key)) {
                    (Some(bv), Some(av)) => structural_delta(bv, av, child, out),
                    (Some(bv), None) => out.push(format!("- {child}: {bv}")),
                    (None, Some(av)) => out.push(format!("+ {child}: {av}")),
                    (None, None) => {}
                }
            }
        }
        (JsonValue::Arr(a), JsonValue::Arr(b)) => {
            for index in 0..a.len().max(b.len()) {
                let child = format!("{path}[{index}]");
                match (a.get(index), b.get(index)) {
                    (Some(bv), Some(av)) => structural_delta(bv, av, child, out),
                    (Some(bv), None) => out.push(format!("- {child}: {bv}")),
                    (None, Some(av)) => out.push(format!("+ {child}: {av}")),
                    (None, None) => {}
                }
            }
        }
        _ => {
            let label = if path.is_empty() {
                "value".to_string()
            } else {
                path
            };
            out.push(format!("~ {label}: {before} -> {after}"));
        }
    }
}

/// Fail-closed diff precondition violation (spec §16.3).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DiffError {
    /// A compiled query references an operation missing from the catalog side.
    UnresolvedOperation { query_id: String, reference: String },
}

impl fmt::Display for DiffError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiffError::UnresolvedOperation {
                query_id,
                reference,
            } => write!(
                f,
                "query `{query_id}` references unresolved operation `{reference}`; diff is incomplete and must fail closed"
            ),
        }
    }
}

struct PageInput {
    route: String,
    state: BTreeMap<String, StateValueIr>,
    components: Vec<ComponentIr>,
}

struct QueryInput {
    alias: String,
    reference: String,
    input: BTreeMap<String, String>,
    execution: ExecutionModeIr,
    required_scopes: Vec<String>,
    output: JsonValue,
}

struct DiffInput {
    app_name: String,
    spec_version: u32,
    modules: BTreeSet<String>,
    pages: BTreeMap<String, PageInput>,
    queries: BTreeMap<String, QueryInput>,
    capability_parameters: BTreeMap<String, Vec<OperationParameter>>,
}

fn resolve(ir: &AppIr, catalog: &CapabilityCatalog) -> Result<DiffInput, DiffError> {
    let mut modules = BTreeSet::new();
    let mut pages = BTreeMap::new();
    let mut queries = BTreeMap::new();
    let mut capability_parameters = BTreeMap::new();

    for module in &ir.modules {
        modules.insert(module.id.as_str().to_string());
        for page in &module.pages {
            let page_key = format!("{}.{}", module.id.as_str(), page.id);
            pages.insert(
                page_key.clone(),
                PageInput {
                    route: page.route.clone(),
                    state: page
                        .state
                        .iter()
                        .map(|state| (state.id.clone(), state.initial.clone()))
                        .collect(),
                    components: page.components.clone(),
                },
            );

            for query in &page.queries {
                let reference = format!("{}.{}", query.provider_id, query.external_operation_id);
                let provider = catalog.providers.get(&query.provider_id).ok_or_else(|| {
                    DiffError::UnresolvedOperation {
                        query_id: query.id.clone(),
                        reference: reference.clone(),
                    }
                })?;
                let operation = provider
                    .operations
                    .get(&query.external_operation_id)
                    .ok_or_else(|| DiffError::UnresolvedOperation {
                        query_id: query.id.clone(),
                        reference: reference.clone(),
                    })?;

                capability_parameters
                    .entry(format!("capability.{reference}"))
                    .or_insert_with(|| operation.parameters.clone());

                queries.insert(
                    query.id.clone(),
                    QueryInput {
                        alias: query.alias.clone(),
                        reference,
                        input: query
                            .input
                            .iter()
                            .map(|binding| (binding.name.clone(), binding.expression.clone()))
                            .collect(),
                        execution: query.execution,
                        required_scopes: query.required_scopes.clone(),
                        output: shape_json(&query.output),
                    },
                );
            }
        }
    }

    Ok(DiffInput {
        app_name: ir.app_name.clone(),
        spec_version: ir.spec_version,
        modules,
        pages,
        queries,
        capability_parameters,
    })
}

/// Diff two compiled plans with their capability catalogs.
///
/// # Errors
///
/// Fails closed with [`DiffError::UnresolvedOperation`] when either side
/// references an operation missing from its catalog (spec §16.3: the diff
/// MUST represent every semantic change, so an unresolvable side is unusable).
pub fn diff_plans(
    before_ir: &AppIr,
    before_catalog: &CapabilityCatalog,
    after_ir: &AppIr,
    after_catalog: &CapabilityCatalog,
) -> Result<DiffReport, DiffError> {
    let before = resolve(before_ir, before_catalog)?;
    let after = resolve(after_ir, after_catalog)?;

    let mut rows = Vec::new();
    diff_app(&before, &after, &mut rows);
    diff_pages(&before, &after, &mut rows);
    diff_queries(&before, &after, &mut rows);
    diff_capabilities(&before, &after, &mut rows);

    rows.sort_by(|a, b| {
        (
            a.kind.as_str(),
            a.target.as_str(),
            a.before.as_ref().map(ToString::to_string),
            a.after.as_ref().map(ToString::to_string),
        )
            .cmp(&(
                b.kind.as_str(),
                b.target.as_str(),
                b.before.as_ref().map(ToString::to_string),
                b.after.as_ref().map(ToString::to_string),
            ))
    });
    rows.dedup();

    let status = if rows.is_empty() {
        DiffStatus::Clean
    } else {
        DiffStatus::Changed
    };
    Ok(DiffReport { status, rows })
}

fn row(
    dimension: DiffDimension,
    target: String,
    before: Option<JsonValue>,
    after: Option<JsonValue>,
    security_significant: bool,
) -> DiffRow {
    DiffRow {
        kind: dimension.as_str().into(),
        target,
        before,
        after,
        security_significant,
    }
}

fn diff_app(before: &DiffInput, after: &DiffInput, rows: &mut Vec<DiffRow>) {
    if before.app_name != after.app_name {
        rows.push(row(
            DiffDimension::AppNameChanged,
            "app".into(),
            Some(JsonValue::Str(before.app_name.clone())),
            Some(JsonValue::Str(after.app_name.clone())),
            false,
        ));
    }
    if before.spec_version != after.spec_version {
        rows.push(row(
            DiffDimension::AppSpecVersionChanged,
            "app".into(),
            Some(JsonValue::Int(i64::from(before.spec_version))),
            Some(JsonValue::Int(i64::from(after.spec_version))),
            false,
        ));
    }
}

fn diff_pages(before: &DiffInput, after: &DiffInput, rows: &mut Vec<DiffRow>) {
    for module in before.modules.difference(&after.modules) {
        rows.push(row(
            DiffDimension::ModuleRemoved,
            module.clone(),
            Some(JsonValue::Str(module.clone())),
            None,
            false,
        ));
    }
    for module in after.modules.difference(&before.modules) {
        rows.push(row(
            DiffDimension::ModuleAdded,
            module.clone(),
            None,
            Some(JsonValue::Str(module.clone())),
            false,
        ));
    }

    let page_keys: BTreeSet<String> = before
        .pages
        .keys()
        .chain(after.pages.keys())
        .cloned()
        .collect();
    for key in page_keys {
        match (before.pages.get(&key), after.pages.get(&key)) {
            (Some(before_page), Some(after_page)) => {
                if before_page.route != after_page.route {
                    rows.push(row(
                        DiffDimension::PageRouteChanged,
                        key.clone(),
                        Some(JsonValue::Str(before_page.route.clone())),
                        Some(JsonValue::Str(after_page.route.clone())),
                        false,
                    ));
                }
                diff_state(&key, before_page, after_page, rows);
                diff_components(&key, &before_page.components, &after_page.components, rows);
            }
            (Some(before_page), None) => rows.push(row(
                DiffDimension::PageRemoved,
                key.clone(),
                Some(page_json(before_page)),
                None,
                false,
            )),
            (None, Some(after_page)) => rows.push(row(
                DiffDimension::PageAdded,
                key.clone(),
                None,
                Some(page_json(after_page)),
                false,
            )),
            (None, None) => {}
        }
    }
}

fn page_json(page: &PageInput) -> JsonValue {
    JsonValue::Obj(vec![("route", JsonValue::Str(page.route.clone()))])
}

fn diff_state(page_key: &str, before: &PageInput, after: &PageInput, rows: &mut Vec<DiffRow>) {
    let state_ids: BTreeSet<&String> = before.state.keys().chain(after.state.keys()).collect();
    for id in state_ids {
        let target = format!("{page_key}.state.{id}");
        match (before.state.get(id), after.state.get(id)) {
            (Some(before_value), Some(after_value)) => {
                if before_value != after_value {
                    rows.push(row(
                        DiffDimension::StateInitialChanged,
                        target,
                        Some(state_value_json(before_value)),
                        Some(state_value_json(after_value)),
                        false,
                    ));
                }
            }
            (Some(before_value), None) => rows.push(row(
                DiffDimension::StateRemoved,
                target,
                Some(state_value_json(before_value)),
                None,
                false,
            )),
            (None, Some(after_value)) => rows.push(row(
                DiffDimension::StateAdded,
                target,
                None,
                Some(state_value_json(after_value)),
                false,
            )),
            (None, None) => {}
        }
    }
}

fn state_value_json(value: &StateValueIr) -> JsonValue {
    match value {
        StateValueIr::String(value) => JsonValue::Str(value.clone()),
        StateValueIr::Integer(value) => JsonValue::Int(*value),
        StateValueIr::Null => JsonValue::Null,
    }
}

fn diff_components(
    page_key: &str,
    before: &[ComponentIr],
    after: &[ComponentIr],
    rows: &mut Vec<DiffRow>,
) {
    let before_by_id: BTreeMap<&str, &ComponentIr> =
        before.iter().map(|c| (c.id.as_str(), c)).collect();
    let after_by_id: BTreeMap<&str, &ComponentIr> =
        after.iter().map(|c| (c.id.as_str(), c)).collect();

    let ids: BTreeSet<&str> = before_by_id
        .keys()
        .chain(after_by_id.keys())
        .copied()
        .collect();
    for id in ids {
        let target = format!("{page_key}.{id}");
        match (before_by_id.get(id), after_by_id.get(id)) {
            (Some(before_component), Some(after_component)) => {
                diff_component(target, before_component, after_component, rows);
            }
            (Some(before_component), None) => rows.push(row(
                DiffDimension::ComponentRemoved,
                target,
                Some(component_json(before_component)),
                None,
                false,
            )),
            (None, Some(after_component)) => rows.push(row(
                DiffDimension::ComponentAdded,
                target,
                None,
                Some(component_json(after_component)),
                false,
            )),
            (None, None) => {}
        }
    }

    // Render order is semantic: compare the relative order of survivor ids.
    let before_survivors: Vec<&str> = before
        .iter()
        .map(|c| c.id.as_str())
        .filter(|id| after_by_id.contains_key(id))
        .collect();
    let after_survivors: Vec<&str> = after
        .iter()
        .map(|c| c.id.as_str())
        .filter(|id| before_by_id.contains_key(id))
        .collect();
    if before_survivors != after_survivors {
        rows.push(row(
            DiffDimension::ComponentReordered,
            page_key.to_string(),
            Some(JsonValue::Arr(
                before_survivors
                    .into_iter()
                    .map(|id| JsonValue::Str(id.into()))
                    .collect(),
            )),
            Some(JsonValue::Arr(
                after_survivors
                    .into_iter()
                    .map(|id| JsonValue::Str(id.into()))
                    .collect(),
            )),
            false,
        ));
    }
}

fn diff_component(
    target: String,
    before: &ComponentIr,
    after: &ComponentIr,
    rows: &mut Vec<DiffRow>,
) {
    if mem::discriminant(&before.kind) != mem::discriminant(&after.kind) {
        rows.push(row(
            DiffDimension::ComponentKindChanged,
            target,
            Some(component_json(before)),
            Some(component_json(after)),
            false,
        ));
        return;
    }

    if let (
        ComponentKindIr::Select {
            options: before_options,
            ..
        },
        ComponentKindIr::Select {
            options: after_options,
            ..
        },
    ) = (&before.kind, &after.kind)
    {
        if before_options != after_options {
            rows.push(row(
                DiffDimension::ComponentOptionsChanged,
                target,
                Some(options_json(before_options)),
                Some(options_json(after_options)),
                false,
            ));
            return;
        }
    }

    if before.kind != after.kind {
        rows.push(row(
            DiffDimension::ComponentFieldChanged,
            target,
            Some(component_json(before)),
            Some(component_json(after)),
            false,
        ));
    }
}

fn options_json(options: &[uiko_core::SelectOptionIr]) -> JsonValue {
    JsonValue::Arr(
        options
            .iter()
            .map(|option| {
                JsonValue::Obj(vec![
                    ("label", JsonValue::Str(option.label.clone())),
                    ("value", state_value_json(&option.value)),
                ])
            })
            .collect(),
    )
}

fn component_json(component: &ComponentIr) -> JsonValue {
    let id = JsonValue::Str(component.id.clone());
    match &component.kind {
        ComponentKindIr::Text { value } => JsonValue::Obj(vec![
            ("id", id),
            ("kind", JsonValue::Str("Text".into())),
            ("value", JsonValue::Str(value.clone())),
        ]),
        ComponentKindIr::Field {
            label,
            binding,
            fallback,
        } => {
            let mut properties = vec![
                ("id", id),
                ("kind", JsonValue::Str("Field".into())),
                ("label", JsonValue::Str(label.clone())),
                ("binding", JsonValue::Str(binding.clone())),
            ];
            if let Some(fallback) = fallback {
                properties.push(("fallback", JsonValue::Str(fallback.clone())));
            }
            JsonValue::Obj(properties)
        }
        ComponentKindIr::Table { binding } => JsonValue::Obj(vec![
            ("id", id),
            ("kind", JsonValue::Str("Table".into())),
            ("binding", JsonValue::Str(binding.clone())),
        ]),
        ComponentKindIr::Select {
            label,
            state,
            options,
        } => JsonValue::Obj(vec![
            ("id", id),
            ("kind", JsonValue::Str("Select".into())),
            ("label", JsonValue::Str(label.clone())),
            ("state", JsonValue::Str(state.clone())),
            ("options", options_json(options)),
        ]),
        ComponentKindIr::Pagination {
            state,
            page_binding,
            page_size_binding,
            total_binding,
        } => JsonValue::Obj(vec![
            ("id", id),
            ("kind", JsonValue::Str("Pagination".into())),
            ("state", JsonValue::Str(state.clone())),
            ("page", JsonValue::Str(page_binding.clone())),
            ("pageSize", JsonValue::Str(page_size_binding.clone())),
            ("total", JsonValue::Str(total_binding.clone())),
        ]),
    }
}

fn diff_queries(before: &DiffInput, after: &DiffInput, rows: &mut Vec<DiffRow>) {
    let query_ids: BTreeSet<&String> = before.queries.keys().chain(after.queries.keys()).collect();
    for id in query_ids {
        match (before.queries.get(id), after.queries.get(id)) {
            (Some(before_query), Some(after_query)) => {
                diff_query(id, before_query, after_query, rows);
            }
            (Some(before_query), None) => rows.push(row(
                DiffDimension::QueryRemoved,
                id.clone(),
                Some(query_json(before_query)),
                None,
                false,
            )),
            (None, Some(after_query)) => rows.push(row(
                DiffDimension::QueryAdded,
                id.clone(),
                None,
                Some(query_json(after_query)),
                false,
            )),
            (None, None) => {}
        }
    }
}

fn query_json(query: &QueryInput) -> JsonValue {
    JsonValue::Obj(vec![
        ("alias", JsonValue::Str(query.alias.clone())),
        ("operation", JsonValue::Str(query.reference.clone())),
    ])
}

fn diff_query(query_id: &str, before: &QueryInput, after: &QueryInput, rows: &mut Vec<DiffRow>) {
    if before.reference != after.reference {
        rows.push(row(
            DiffDimension::QueryOperationRemapped,
            query_id.to_string(),
            Some(operation_json(&before.reference)),
            Some(operation_json(&after.reference)),
            true,
        ));
    }

    if before.execution != after.execution {
        let downgrade = matches!(
            (before.execution, after.execution),
            (ExecutionModeIr::Managed, ExecutionModeIr::Unmanaged)
        );
        let mut after_properties = vec![(
            "execution",
            JsonValue::Str(mode_str(after.execution).into()),
        )];
        if downgrade {
            // §16.3: the after payload names the controls no longer claimed.
            after_properties.push(("controlsNoLongerClaimed", JsonValue::Bool(true)));
        }
        rows.push(row(
            DiffDimension::QueryExecutionChanged,
            query_id.to_string(),
            Some(JsonValue::Obj(vec![(
                "execution",
                JsonValue::Str(mode_str(before.execution).into()),
            )])),
            Some(JsonValue::Obj(after_properties)),
            downgrade,
        ));
    }

    // Compare as sets: the widening rule is set-based, so scope order alone
    // (possible for programmatic callers) must not read as a change.
    if before.required_scopes.iter().collect::<BTreeSet<&String>>()
        != after.required_scopes.iter().collect::<BTreeSet<&String>>()
    {
        // §17.2 widening: removing a required scope admits more principals.
        let after_scopes: BTreeSet<&String> = after.required_scopes.iter().collect();
        let widening = before
            .required_scopes
            .iter()
            .any(|scope| !after_scopes.contains(scope));
        rows.push(row(
            DiffDimension::QueryAuthorizationChanged,
            query_id.to_string(),
            Some(scopes_json(&before.required_scopes)),
            Some(scopes_json(&after.required_scopes)),
            widening,
        ));
    }

    let names: BTreeSet<&String> = before.input.keys().chain(after.input.keys()).collect();
    for name in names {
        let target = format!("{query_id}.input.{name}");
        match (before.input.get(name), after.input.get(name)) {
            (Some(before_expression), Some(after_expression)) => {
                if before_expression != after_expression {
                    rows.push(row(
                        DiffDimension::QueryInputRebound,
                        target,
                        Some(JsonValue::Str(before_expression.clone())),
                        Some(JsonValue::Str(after_expression.clone())),
                        false,
                    ));
                }
            }
            (Some(before_expression), None) => rows.push(row(
                DiffDimension::QueryInputRemoved,
                target,
                Some(JsonValue::Str(before_expression.clone())),
                None,
                false,
            )),
            (None, Some(after_expression)) => rows.push(row(
                DiffDimension::QueryInputAdded,
                target,
                None,
                Some(JsonValue::Str(after_expression.clone())),
                false,
            )),
            (None, None) => {}
        }
    }

    if before.output != after.output {
        rows.push(row(
            DiffDimension::QueryOutputChanged,
            query_id.to_string(),
            Some(before.output.clone()),
            Some(after.output.clone()),
            true,
        ));
    }
}

fn operation_json(reference: &str) -> JsonValue {
    JsonValue::Obj(vec![("operation", JsonValue::Str(reference.into()))])
}

fn scopes_json(scopes: &[String]) -> JsonValue {
    JsonValue::Arr(
        scopes
            .iter()
            .map(|scope| JsonValue::Str(scope.clone()))
            .collect(),
    )
}

fn mode_str(mode: ExecutionModeIr) -> &'static str {
    match mode {
        ExecutionModeIr::Managed => "managed",
        ExecutionModeIr::Unmanaged => "unmanaged",
    }
}

/// A capability present on one side only is already covered by the
/// `query.added`/`query.removed` row that (de)referenced it, so parameters
/// are compared over the referenced-on-both-sides intersection.
fn diff_capabilities(before: &DiffInput, after: &DiffInput, rows: &mut Vec<DiffRow>) {
    for (key, before_parameters) in &before.capability_parameters {
        let Some(after_parameters) = after.capability_parameters.get(key) else {
            continue;
        };
        if before_parameters == after_parameters {
            continue;
        }

        // Security-significant iff a before-required parameter is lost or
        // becomes optional: fewer enforced preconditions.
        let after_by_name: BTreeMap<&str, &OperationParameter> = after_parameters
            .iter()
            .map(|parameter| (parameter.name.as_str(), parameter))
            .collect();
        let loses_required = before_parameters.iter().any(|parameter| {
            parameter.required
                && after_by_name
                    .get(parameter.name.as_str())
                    .is_none_or(|after| !after.required)
        });
        rows.push(row(
            DiffDimension::CapabilityParametersChanged,
            key.clone(),
            Some(parameters_json(before_parameters)),
            Some(parameters_json(after_parameters)),
            loses_required,
        ));
    }
}

fn parameters_json(parameters: &[OperationParameter]) -> JsonValue {
    JsonValue::Arr(
        parameters
            .iter()
            .map(|parameter| {
                JsonValue::Obj(vec![
                    ("name", JsonValue::Str(parameter.name.clone())),
                    (
                        "location",
                        JsonValue::Str(location_str(parameter.location).into()),
                    ),
                    ("required", JsonValue::Bool(parameter.required)),
                ])
            })
            .collect(),
    )
}

fn location_str(location: uiko_capabilities::ParameterLocation) -> &'static str {
    match location {
        uiko_capabilities::ParameterLocation::Path => "Path",
        uiko_capabilities::ParameterLocation::Query => "Query",
    }
}

fn shape_json(shape: &ValueShape) -> JsonValue {
    JsonValue::Obj(vec![
        ("nullable", JsonValue::Bool(shape.nullable)),
        ("kind", kind_json(&shape.kind)),
    ])
}

fn kind_json(kind: &ValueKind) -> JsonValue {
    match kind {
        ValueKind::String => JsonValue::Str("String".into()),
        ValueKind::Integer => JsonValue::Str("Integer".into()),
        ValueKind::Number => JsonValue::Str("Number".into()),
        ValueKind::Boolean => JsonValue::Str("Boolean".into()),
        ValueKind::Array(item) => JsonValue::Obj(vec![("array", shape_json(item))]),
        ValueKind::Object(fields) => JsonValue::Obj(vec![(
            "object",
            JsonValue::Arr(
                fields
                    .iter()
                    .map(|(name, field)| {
                        JsonValue::Obj(vec![
                            ("name", JsonValue::Str(name.clone())),
                            ("required", JsonValue::Bool(field.required)),
                            ("value", shape_json(&field.value)),
                        ])
                    })
                    .collect(),
            ),
        )]),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use uiko_capabilities::{
        CapabilityCatalog, CapabilityProvider, ObjectField, OperationParameter, ParameterLocation,
        QueryOperation, ValueKind, ValueShape,
    };
    use uiko_core::{
        AppIr, ComponentIr, ComponentKindIr, ExecutionModeIr, ModuleId, ModuleIr, PageIr,
        PageStateIr, QueryInputIr, QueryIr, SelectOptionIr, StateValueIr,
    };

    use super::{DiffDimension, diff_plans};

    fn string_shape() -> ValueShape {
        ValueShape {
            nullable: false,
            kind: ValueKind::String,
        }
    }

    fn list_output() -> ValueShape {
        ValueShape {
            nullable: false,
            kind: ValueKind::Object(BTreeMap::from([
                (
                    "total".into(),
                    ObjectField {
                        required: true,
                        value: ValueShape {
                            nullable: false,
                            kind: ValueKind::Integer,
                        },
                    },
                ),
                (
                    "items".into(),
                    ObjectField {
                        required: true,
                        value: ValueShape {
                            nullable: false,
                            kind: ValueKind::Array(Box::new(string_shape())),
                        },
                    },
                ),
            ])),
        }
    }

    fn customer_output() -> ValueShape {
        ValueShape {
            nullable: false,
            kind: ValueKind::Object(BTreeMap::from([(
                "name".into(),
                ObjectField {
                    required: true,
                    value: string_shape(),
                },
            )])),
        }
    }

    fn catalog() -> CapabilityCatalog {
        CapabilityCatalog {
            providers: BTreeMap::from([
                (
                    "crm".into(),
                    CapabilityProvider {
                        id: "crm".into(),
                        operations: BTreeMap::from([
                            (
                                "getCustomer".into(),
                                QueryOperation {
                                    external_id: "getCustomer".into(),
                                    parameters: vec![OperationParameter {
                                        name: "customerId".into(),
                                        location: ParameterLocation::Path,
                                        required: true,
                                    }],
                                    output: customer_output(),
                                },
                            ),
                            (
                                "listCustomers".into(),
                                QueryOperation {
                                    external_id: "listCustomers".into(),
                                    parameters: vec![
                                        OperationParameter {
                                            name: "status".into(),
                                            location: ParameterLocation::Query,
                                            required: false,
                                        },
                                        OperationParameter {
                                            name: "page".into(),
                                            location: ParameterLocation::Query,
                                            required: false,
                                        },
                                    ],
                                    output: list_output(),
                                },
                            ),
                        ]),
                    },
                ),
                (
                    "billing".into(),
                    CapabilityProvider {
                        id: "billing".into(),
                        operations: BTreeMap::from([(
                            "getInvoice".into(),
                            QueryOperation {
                                external_id: "getInvoice".into(),
                                parameters: vec![OperationParameter {
                                    name: "invoiceId".into(),
                                    location: ParameterLocation::Path,
                                    required: true,
                                }],
                                output: string_shape(),
                            },
                        )]),
                    },
                ),
            ]),
        }
    }

    fn query(
        id: &str,
        alias: &str,
        reference: (&str, &str),
        input: &[(&str, &str)],
        execution: ExecutionModeIr,
        scopes: &[&str],
        output: ValueShape,
    ) -> QueryIr {
        QueryIr {
            id: id.into(),
            alias: alias.into(),
            provider_id: reference.0.into(),
            external_operation_id: reference.1.into(),
            input: input
                .iter()
                .map(|(name, expression)| QueryInputIr {
                    name: (*name).into(),
                    expression: (*expression).into(),
                })
                .collect(),
            execution,
            required_scopes: scopes.iter().map(|scope| (*scope).to_string()).collect(),
            output,
        }
    }

    /// In-memory corpus: one module, two pages, three queries, all five
    /// component kinds, one unmanaged + scoped query.
    #[expect(
        clippy::too_many_lines,
        reason = "one fixture builder reads better than split partial builders"
    )]
    fn corpus() -> AppIr {
        let list_page = PageIr {
            id: "CustomerList".into(),
            route: "/customers".into(),
            state: vec![
                PageStateIr {
                    id: "status".into(),
                    initial: StateValueIr::Null,
                },
                PageStateIr {
                    id: "page".into(),
                    initial: StateValueIr::Integer(1),
                },
            ],
            queries: vec![
                query(
                    "customers.CustomerList.query.customers",
                    "customers",
                    ("crm", "listCustomers"),
                    &[("status", "state.status"), ("page", "state.page")],
                    ExecutionModeIr::Unmanaged,
                    &["crm:list"],
                    list_output(),
                ),
                query(
                    "customers.CustomerList.query.invoice",
                    "invoice",
                    ("billing", "getInvoice"),
                    &[("invoiceId", "state.status")],
                    ExecutionModeIr::Managed,
                    &["billing:read"],
                    string_shape(),
                ),
            ],
            components: vec![
                ComponentIr {
                    id: "status".into(),
                    kind: ComponentKindIr::Select {
                        label: "Status".into(),
                        state: "status".into(),
                        options: vec![
                            SelectOptionIr {
                                label: "All".into(),
                                value: StateValueIr::Null,
                            },
                            SelectOptionIr {
                                label: "Active".into(),
                                value: StateValueIr::String("active".into()),
                            },
                        ],
                    },
                },
                ComponentIr {
                    id: "table".into(),
                    kind: ComponentKindIr::Table {
                        binding: "customers.items".into(),
                    },
                },
                ComponentIr {
                    id: "pager".into(),
                    kind: ComponentKindIr::Pagination {
                        state: "page".into(),
                        page_binding: "customers.page".into(),
                        page_size_binding: "customers.pageSize".into(),
                        total_binding: "customers.total".into(),
                    },
                },
            ],
        };

        let detail_page = PageIr {
            id: "CustomerDetail".into(),
            route: "/customers/:customerId".into(),
            state: Vec::new(),
            queries: vec![query(
                "customers.CustomerDetail.query.customer",
                "customer",
                ("crm", "getCustomer"),
                &[("customerId", "route.customerId")],
                ExecutionModeIr::Managed,
                &["crm:read"],
                customer_output(),
            )],
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
                        fallback: None,
                    },
                },
            ],
        };

        AppIr {
            app_name: "support-console".into(),
            spec_version: 1,
            modules: vec![ModuleIr {
                id: ModuleId::new("customers"),
                pages: vec![list_page, detail_page],
            }],
        }
    }

    fn diff(before: &AppIr, after: &AppIr) -> super::DiffReport {
        diff_plans(before, &catalog(), after, &catalog()).expect("resolvable corpus")
    }

    fn kinds(report: &super::DiffReport) -> Vec<&str> {
        report.rows.iter().map(|row| row.kind.as_str()).collect()
    }

    #[test]
    fn identical_plans_diff_clean() {
        let report = diff(&corpus(), &corpus());

        assert_eq!(report.status, super::DiffStatus::Clean);
        assert!(report.rows.is_empty());
        assert_eq!(report.to_json(), r#"{"status":"clean","rows":[]}"#);
        assert_eq!(report.to_human(), "uiko diff: clean\n");
    }

    #[test]
    fn output_is_byte_deterministic() {
        let mut after = corpus();
        after.app_name = "renamed".into();
        after.modules[0].pages[0].route = "/clients".into();

        let first = diff(&corpus(), &after);
        let second = diff(&corpus(), &after);

        assert_eq!(first.to_json(), second.to_json());
        assert_eq!(first.to_human(), second.to_human());
        assert_eq!(
            first.to_json(),
            r#"{"status":"changed","rows":[{"kind":"app.name.changed","target":"app","securitySignificant":false,"before":"support-console","after":"renamed"},{"kind":"page.route.changed","target":"customers.CustomerList","securitySignificant":false,"before":"/customers","after":"/clients"}]}"#
        );
    }

    #[test]
    fn structured_row_renders_field_level_delta() {
        use super::{DiffReport, DiffRow, DiffStatus, JsonValue};

        let leaf = |required: bool| {
            JsonValue::Obj(vec![
                (
                    "kind",
                    JsonValue::Obj(vec![(
                        "object",
                        JsonValue::Arr(vec![JsonValue::Obj(vec![
                            ("name", JsonValue::Str("total".into())),
                            ("required", JsonValue::Bool(required)),
                        ])]),
                    )]),
                ),
                ("nullable", JsonValue::Bool(false)),
            ])
        };
        let report = DiffReport {
            status: DiffStatus::Changed,
            rows: vec![DiffRow {
                kind: "query.output.changed".into(),
                target: "customers.CustomerList.query.customers".into(),
                before: Some(leaf(true)),
                after: Some(leaf(false)),
                security_significant: true,
            }],
        };

        let human = report.to_human();
        assert!(human.contains("  changes:\n"), "{human}");
        assert!(
            human.contains("~ kind.object[0].required: true -> false"),
            "{human}"
        );
        // The opaque single-line blob must not be printed for structured rows.
        assert!(!human.contains("  before: {"), "{human}");
        assert!(!human.contains("  after: {"), "{human}");
        assert!(human.contains("security-significant"), "{human}");
    }

    #[test]
    fn array_element_removal_renders_minus_line() {
        use super::{DiffReport, DiffRow, DiffStatus, JsonValue};

        let report = DiffReport {
            status: DiffStatus::Changed,
            rows: vec![DiffRow {
                kind: "component.options.changed".into(),
                target: "customers.CustomerList.status".into(),
                before: Some(JsonValue::Arr(vec![
                    JsonValue::Str("active".into()),
                    JsonValue::Str("inactive".into()),
                ])),
                after: Some(JsonValue::Arr(vec![JsonValue::Str("active".into())])),
                security_significant: false,
            }],
        };

        let human = report.to_human();
        assert!(human.contains(r#"- [1]: "inactive""#), "{human}");
    }

    #[test]
    fn scalar_row_keeps_plain_before_after() {
        use super::{DiffReport, DiffRow, DiffStatus, JsonValue};

        let report = DiffReport {
            status: DiffStatus::Changed,
            rows: vec![DiffRow {
                kind: "page.route.changed".into(),
                target: "customers.CustomerDetail".into(),
                before: Some(JsonValue::Str("/customers/:customerId".into())),
                after: Some(JsonValue::Str("/accounts/:customerId".into())),
                security_significant: false,
            }],
        };

        let human = report.to_human();
        assert!(
            human.contains("  before: \"/customers/:customerId\""),
            "{human}"
        );
        assert!(
            human.contains("  after: \"/accounts/:customerId\""),
            "{human}"
        );
        assert!(!human.contains("changes:"), "{human}");
    }

    #[test]
    fn id_reorder_is_not_a_change() {
        let mut after = corpus();
        after.modules[0].pages.reverse();
        let list = &mut after.modules[0].pages[1];
        list.state.reverse();
        list.queries.reverse();

        let report = diff(&corpus(), &after);

        assert_eq!(report.status, super::DiffStatus::Clean);
    }

    #[test]
    fn component_survivor_swap_is_exactly_one_reorder_row() {
        let mut after = corpus();
        let page = &mut after.modules[0].pages[1];
        page.components.reverse();

        let report = diff(&corpus(), &after);

        assert_eq!(
            kinds(&report),
            vec![DiffDimension::ComponentReordered.as_str()]
        );
        assert_eq!(report.rows[0].target, "customers.CustomerDetail");
        assert!(!report.rows[0].security_significant);
    }

    #[test]
    fn insertion_among_survivors_is_not_reorder() {
        let mut after = corpus();
        after.modules[0].pages[1].components.insert(
            1,
            ComponentIr {
                id: "badge".into(),
                kind: ComponentKindIr::Text {
                    value: "VIP".into(),
                },
            },
        );

        let report = diff(&corpus(), &after);

        assert_eq!(kinds(&report), vec![DiffDimension::ComponentAdded.as_str()]);
        assert_eq!(report.rows[0].target, "customers.CustomerDetail.badge");
    }

    #[test]
    fn select_options_order_is_semantic() {
        let mut after = corpus();
        let ComponentKindIr::Select { options, .. } =
            &mut after.modules[0].pages[0].components[0].kind
        else {
            panic!("expected Select");
        };
        options.reverse();

        let report = diff(&corpus(), &after);

        assert_eq!(
            kinds(&report),
            vec![DiffDimension::ComponentOptionsChanged.as_str()]
        );
        assert_eq!(report.rows[0].target, "customers.CustomerList.status");
    }

    #[test]
    fn unresolved_operation_fails_closed() {
        let mut broken_catalog = catalog();
        broken_catalog.providers.remove("billing");

        let error =
            super::diff_plans(&corpus(), &broken_catalog, &corpus(), &catalog()).unwrap_err();

        assert_eq!(
            error,
            super::DiffError::UnresolvedOperation {
                query_id: "customers.CustomerList.query.invoice".into(),
                reference: "billing.getInvoice".into(),
            }
        );
    }

    type Mutation = Box<dyn Fn(&mut AppIr, &mut CapabilityCatalog)>;

    /// Completeness property: every taxonomy kind has a representing mutation
    /// the diff must surface (spec §16.3: every semantic change is a row).
    #[expect(
        clippy::too_many_lines,
        reason = "one entry per taxonomy kind is the point of the table"
    )]
    #[test]
    fn mutation_table_covers_every_taxonomy_kind() {
        let mutations: Vec<(&str, &str, Mutation)> = vec![
            (
                "app.name.changed",
                "app",
                Box::new(|ir, _| ir.app_name = "renamed".into()),
            ),
            (
                "app.spec_version.changed",
                "app",
                Box::new(|ir, _| ir.spec_version = 2),
            ),
            (
                "module.added",
                "billing",
                Box::new(|ir, _| {
                    ir.modules.push(ModuleIr {
                        id: ModuleId::new("billing"),
                        pages: Vec::new(),
                    });
                }),
            ),
            (
                "module.removed",
                "customers",
                Box::new(|ir, _| {
                    ir.modules.clear();
                }),
            ),
            (
                "page.added",
                "customers.Reports",
                Box::new(|ir, _| {
                    ir.modules[0].pages.push(PageIr {
                        id: "Reports".into(),
                        route: "/reports".into(),
                        state: Vec::new(),
                        queries: Vec::new(),
                        components: Vec::new(),
                    });
                }),
            ),
            (
                "page.removed",
                "customers.CustomerDetail",
                Box::new(|ir, _| {
                    ir.modules[0].pages.remove(1);
                }),
            ),
            (
                "page.route.changed",
                "customers.CustomerList",
                Box::new(|ir, _| ir.modules[0].pages[0].route = "/clients".into()),
            ),
            (
                "state.added",
                "customers.CustomerList.state.filter",
                Box::new(|ir, _| {
                    ir.modules[0].pages[0].state.push(PageStateIr {
                        id: "filter".into(),
                        initial: StateValueIr::String("all".into()),
                    });
                }),
            ),
            (
                "state.removed",
                "customers.CustomerList.state.status",
                Box::new(|ir, _| {
                    ir.modules[0].pages[0]
                        .state
                        .retain(|state| state.id != "status");
                }),
            ),
            (
                "state.initial.changed",
                "customers.CustomerList.state.page",
                Box::new(|ir, _| {
                    let state = &mut ir.modules[0].pages[0].state[1];
                    state.initial = StateValueIr::Integer(2);
                }),
            ),
            (
                "query.added",
                "customers.CustomerList.query.aging",
                Box::new(|ir, _| {
                    ir.modules[0].pages[0].queries.push(query(
                        "customers.CustomerList.query.aging",
                        "aging",
                        ("crm", "listCustomers"),
                        &[],
                        ExecutionModeIr::Managed,
                        &[],
                        list_output(),
                    ));
                }),
            ),
            (
                "query.removed",
                "customers.CustomerList.query.invoice",
                Box::new(|ir, _| {
                    ir.modules[0].pages[0].queries.remove(1);
                }),
            ),
            (
                "query.operation.remapped",
                "customers.CustomerDetail.query.customer",
                Box::new(|ir, _| {
                    let detail = &mut ir.modules[0].pages[1].queries[0];
                    detail.provider_id = "crm".into();
                    detail.external_operation_id = "listCustomers".into();
                }),
            ),
            (
                "query.input.added",
                "customers.CustomerList.query.customers.input.limit",
                Box::new(|ir, _| {
                    ir.modules[0].pages[0].queries[0].input.push(QueryInputIr {
                        name: "limit".into(),
                        expression: "state.page".into(),
                    });
                }),
            ),
            (
                "query.input.removed",
                "customers.CustomerList.query.customers.input.status",
                Box::new(|ir, _| {
                    let customers = &mut ir.modules[0].pages[0].queries[0];
                    customers.input.retain(|binding| binding.name != "status");
                }),
            ),
            (
                "query.input.rebound",
                "customers.CustomerList.query.customers.input.status",
                Box::new(|ir, _| {
                    ir.modules[0].pages[0].queries[0].input[0].expression = "state.filter".into();
                }),
            ),
            (
                "query.output.changed",
                "customers.CustomerDetail.query.customer",
                Box::new(|ir, _| {
                    ir.modules[0].pages[1].queries[0].output = string_shape();
                }),
            ),
            (
                "capability.parameters.changed",
                "capability.crm.getCustomer",
                Box::new(|_, catalog| {
                    let crm = catalog.providers.get_mut("crm").expect("crm provider");
                    let operation = crm.operations.get_mut("getCustomer").expect("operation");
                    operation.parameters[0].required = false;
                }),
            ),
            (
                "query.execution.changed",
                "customers.CustomerDetail.query.customer",
                Box::new(|ir, _| {
                    ir.modules[0].pages[1].queries[0].execution = ExecutionModeIr::Unmanaged;
                }),
            ),
            (
                "query.authorization.changed",
                "customers.CustomerList.query.customers",
                Box::new(|ir, _| {
                    ir.modules[0].pages[0].queries[0].required_scopes =
                        vec!["crm:list".into(), "crm:admin".into()];
                }),
            ),
            (
                "component.added",
                "customers.CustomerList.badge",
                Box::new(|ir, _| {
                    ir.modules[0].pages[0].components.push(ComponentIr {
                        id: "badge".into(),
                        kind: ComponentKindIr::Text {
                            value: "VIP".into(),
                        },
                    });
                }),
            ),
            (
                "component.removed",
                "customers.CustomerList.table",
                Box::new(|ir, _| {
                    ir.modules[0].pages[0]
                        .components
                        .retain(|c| c.id != "table");
                }),
            ),
            (
                "component.kind.changed",
                "customers.CustomerDetail.name",
                Box::new(|ir, _| {
                    ir.modules[0].pages[1].components[1].kind = ComponentKindIr::Table {
                        binding: "customer.name".into(),
                    };
                }),
            ),
            (
                "component.field.changed",
                "customers.CustomerDetail.name",
                Box::new(|ir, _| {
                    let ComponentKindIr::Field { label, .. } =
                        &mut ir.modules[0].pages[1].components[1].kind
                    else {
                        panic!("expected Field");
                    };
                    *label = "Full Name".into();
                }),
            ),
            (
                "component.options.changed",
                "customers.CustomerList.status",
                Box::new(|ir, _| {
                    let ComponentKindIr::Select { options, .. } =
                        &mut ir.modules[0].pages[0].components[0].kind
                    else {
                        panic!("expected Select");
                    };
                    options[1].label = "Live".into();
                }),
            ),
            (
                "component.reordered",
                "customers.CustomerList",
                Box::new(|ir, _| ir.modules[0].pages[0].components.swap(0, 1)),
            ),
        ];

        for (expected_kind, expected_target, apply) in mutations {
            let mut after = corpus();
            let mut after_catalog = catalog();
            apply(&mut after, &mut after_catalog);

            let report = super::diff_plans(&corpus(), &catalog(), &after, &after_catalog)
                .expect("resolvable corpus");
            let seen: Vec<(&str, &str)> = report
                .rows
                .iter()
                .map(|row| (row.kind.as_str(), row.target.as_str()))
                .collect();
            assert!(
                seen.contains(&(expected_kind, expected_target)),
                "expected row `{expected_kind}` for `{expected_target}`, saw {seen:?}"
            );
        }
    }

    #[test]
    fn operation_remap_is_security_significant() {
        let mut after = corpus();
        let detail = &mut after.modules[0].pages[1].queries[0];
        detail.provider_id = "crm".into();
        detail.external_operation_id = "listCustomers".into();

        let report = diff(&corpus(), &after);

        let row = report
            .rows
            .iter()
            .find(|row| row.kind == DiffDimension::QueryOperationRemapped.as_str())
            .expect("remap row");
        assert!(row.security_significant);
    }

    #[test]
    fn execution_downgrade_is_security_significant_and_names_lost_controls() {
        let mut after = corpus();
        after.modules[0].pages[1].queries[0].execution = ExecutionModeIr::Unmanaged;

        let report = diff(&corpus(), &after);

        let row = report
            .rows
            .iter()
            .find(|row| row.kind == DiffDimension::QueryExecutionChanged.as_str())
            .expect("execution row");
        assert!(row.security_significant);
        let after_json = row.after.as_ref().expect("after payload").to_string();
        assert!(after_json.contains("\"controlsNoLongerClaimed\":true"));
    }

    #[test]
    fn execution_upgrade_is_not_security_significant() {
        let mut after = corpus();
        after.modules[0].pages[0].queries[0].execution = ExecutionModeIr::Managed;

        let report = diff(&corpus(), &after);

        let row = report
            .rows
            .iter()
            .find(|row| row.kind == DiffDimension::QueryExecutionChanged.as_str())
            .expect("execution row");
        assert!(!row.security_significant);
    }

    #[test]
    fn authorization_scope_removal_is_widening_and_security_significant() {
        let mut after = corpus();
        after.modules[0].pages[0].queries[0].required_scopes = Vec::new();

        let report = diff(&corpus(), &after);

        let row = report
            .rows
            .iter()
            .find(|row| row.kind == DiffDimension::QueryAuthorizationChanged.as_str())
            .expect("authorization row");
        assert!(row.security_significant);
    }

    #[test]
    fn authorization_scope_addition_is_not_security_significant() {
        let mut after = corpus();
        after.modules[0].pages[0].queries[0].required_scopes =
            vec!["crm:admin".into(), "crm:list".into()];

        let report = diff(&corpus(), &after);

        let row = report
            .rows
            .iter()
            .find(|row| row.kind == DiffDimension::QueryAuthorizationChanged.as_str())
            .expect("authorization row");
        assert!(!row.security_significant);
    }

    #[test]
    fn output_change_is_security_significant() {
        let mut after = corpus();
        after.modules[0].pages[1].queries[0].output = string_shape();

        let report = diff(&corpus(), &after);

        let row = report
            .rows
            .iter()
            .find(|row| row.kind == DiffDimension::QueryOutputChanged.as_str())
            .expect("output row");
        assert!(row.security_significant);
    }

    #[test]
    fn required_parameter_loss_is_security_significant() {
        let mut after_catalog = catalog();
        let crm = after_catalog
            .providers
            .get_mut("crm")
            .expect("crm provider");
        let operation = crm.operations.get_mut("getCustomer").expect("getCustomer");
        operation.parameters[0].required = false;

        let report = super::diff_plans(&corpus(), &catalog(), &corpus(), &after_catalog)
            .expect("resolvable corpus");

        let row = report
            .rows
            .iter()
            .find(|row| row.kind == DiffDimension::CapabilityParametersChanged.as_str())
            .expect("capability row");
        assert_eq!(row.target, "capability.crm.getCustomer");
        assert!(row.security_significant);
    }

    #[test]
    fn optional_parameter_addition_is_not_security_significant() {
        let mut after_catalog = catalog();
        let crm = after_catalog
            .providers
            .get_mut("crm")
            .expect("crm provider");
        let operation = crm
            .operations
            .get_mut("listCustomers")
            .expect("listCustomers");
        operation.parameters.push(OperationParameter {
            name: "limit".into(),
            location: ParameterLocation::Query,
            required: false,
        });

        let report = super::diff_plans(&corpus(), &catalog(), &corpus(), &after_catalog)
            .expect("resolvable corpus");

        let row = report
            .rows
            .iter()
            .find(|row| row.kind == DiffDimension::CapabilityParametersChanged.as_str())
            .expect("capability row");
        assert!(!row.security_significant);
    }
}

#[cfg(test)]
mod fixtures {
    use std::fs;
    use std::path::{Path, PathBuf};

    use uiko_compiler::compile;
    use uiko_core::{Located, SourceId, TextSpan};
    use uiko_project::load_project;
    use uiko_source::AppSource;
    use uiko_source_jsonc::parse_page;

    use super::DiffDimension;

    fn fixture_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../experiments/fixtures/compiler-support-console")
    }

    fn fixture_catalog() -> uiko_capabilities::CapabilityCatalog {
        load_project(&fixture_root())
            .expect("fixture loads")
            .capabilities
    }

    fn compile_fixture(source: &Located<AppSource>) -> uiko_core::AppIr {
        compile(source, &fixture_catalog()).expect("fixture compiles")
    }

    /// Splice one frozen G0 page into the loadable fixture project (the G0
    /// prerequisite dirs have no `integrations/`, so the app + integrations
    /// come from the fixture root).
    fn g0_splice(dir: &str) -> Located<AppSource> {
        let project = load_project(&fixture_root()).expect("fixture loads");
        let page_path = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
            "../../experiments/g0/prerequisites/C_UIKO/{dir}/features/customers/list.jsonc"
        ));
        let text = fs::read_to_string(&page_path).expect("frozen G0 page exists");
        let page = parse_page(&SourceId::new(format!("g0/{dir}/list.jsonc")), &text)
            .expect("frozen G0 page parses");

        let mut source = project.source.value.clone();
        source.modules[0].value.pages = vec![page];
        Located::new(source, project.source.span.clone())
    }

    #[test]
    fn compiler_support_console_self_diff_is_clean() {
        let project = load_project(&fixture_root()).expect("fixture loads");
        let ir = compile(&project.source, &project.capabilities).expect("fixture compiles");

        let report = super::diff_plans(&ir, &project.capabilities, &ir, &project.capabilities)
            .expect("resolvable fixture");

        assert_eq!(report.status, super::DiffStatus::Clean);
    }

    #[test]
    fn g0_frozen_splice_diff_surfaces_state_input_and_component_rows() {
        let before = compile_fixture(&g0_splice("D01"));
        let after = compile_fixture(&g0_splice("D03"));

        let report = super::diff_plans(&before, &fixture_catalog(), &after, &fixture_catalog())
            .expect("resolvable splice");

        let seen: Vec<(&str, &str)> = report
            .rows
            .iter()
            .map(|row| (row.kind.as_str(), row.target.as_str()))
            .collect();
        assert_eq!(
            seen,
            vec![
                (
                    DiffDimension::ComponentAdded.as_str(),
                    "customers.CustomerList.statusFilter"
                ),
                (
                    DiffDimension::QueryInputAdded.as_str(),
                    "customers.CustomerList.query.customers.input.status"
                ),
                (
                    DiffDimension::StateAdded.as_str(),
                    "customers.CustomerList.state.status"
                ),
            ]
        );
    }

    /// Seeded review-relevant mutations at the source-DTO level: operation
    /// remap, authorization widening, execution downgrade.
    #[test]
    fn seeded_mutations_surface_as_security_significant_rows() {
        let project = load_project(&fixture_root()).expect("fixture loads");

        let mut before = project.source.value.clone();
        let detail = &mut before.modules[0].value.pages[1].value.queries[0].value;
        detail.authorization = vec![Located::new(
            "crm:read".into(),
            TextSpan::new(SourceId::new("seed"), 0, 1),
        )];
        // The remap swaps the detail query's output shape, so Field bindings
        // against the old output would not compile on either side. Both
        // revisions keep only the binding-free title component; the seeded
        // rows under test are query-level, not binding-level.
        before.modules[0].value.pages[1]
            .value
            .components
            .truncate(1);

        let mut after = before.clone();
        let detail = &mut after.modules[0].value.pages[1].value.queries[0].value;
        detail.operation.value = "crm.listCustomers".into();
        detail.input.clear();
        detail.authorization = Vec::new();
        detail.execution = Some(Located::new(
            "unmanaged".into(),
            TextSpan::new(SourceId::new("seed"), 0, 1),
        ));

        let before_span = project.source.span.clone();
        let before_ir = compile(
            &Located::new(before, before_span.clone()),
            &project.capabilities,
        )
        .expect("seeded before compiles");
        let after_ir = compile(&Located::new(after, before_span), &project.capabilities)
            .expect("seeded after compiles");

        let report = super::diff_plans(
            &before_ir,
            &project.capabilities,
            &after_ir,
            &project.capabilities,
        )
        .expect("resolvable seeds");

        let seen: Vec<(&str, bool)> = report
            .rows
            .iter()
            .map(|row| (row.kind.as_str(), row.security_significant))
            .collect();
        assert_eq!(
            seen,
            vec![
                (DiffDimension::QueryAuthorizationChanged.as_str(), true),
                (DiffDimension::QueryExecutionChanged.as_str(), true),
                // Consequences of the same remap, not separate seeds:
                (DiffDimension::QueryInputRemoved.as_str(), false),
                (DiffDimension::QueryOperationRemapped.as_str(), true),
                (DiffDimension::QueryOutputChanged.as_str(), true),
            ]
        );
    }

    /// Output shape swap via a mutated catalog: contract fidelity is
    /// security-significant without any source change.
    #[test]
    fn catalog_output_swap_surfaces_security_significant_output_change() {
        let project = load_project(&fixture_root()).expect("fixture loads");
        let ir = compile(&project.source, &project.capabilities).expect("fixture compiles");

        let mut mutated = project.capabilities.clone();
        let crm = mutated.providers.get_mut("crm").expect("crm provider");
        let operation = crm.operations.get_mut("getCustomer").expect("getCustomer");
        // Flip outer nullability: the binding paths (customer.name, …) still
        // resolve, so the source recompiles, but the compiled output shape —
        // snapshotted into the IR at compile time — differs.
        operation.output.nullable = true;

        let after_ir = compile(&project.source, &mutated).expect("mutated catalog recompiles");
        let report =
            super::diff_plans(&ir, &project.capabilities, &after_ir, &mutated).expect("resolvable");

        let row = report
            .rows
            .iter()
            .find(|row| row.kind == DiffDimension::QueryOutputChanged.as_str())
            .expect("output row");
        assert_eq!(row.target, "customers.CustomerDetail.query.customer");
        assert!(row.security_significant);
    }
}
