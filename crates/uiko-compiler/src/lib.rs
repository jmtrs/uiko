#![forbid(unsafe_code)]

use std::collections::BTreeSet;

use uiko_core::{
    AppIr, ComponentIr, Diagnostic, Located, ModuleIr, PageIr, Severity,
};
use uiko_source::{AppSource, ComponentSource, PageSource};

pub const SUPPORTED_SPEC_VERSION: u32 = 1;

/// Compile already-located source DTOs into the first canonical IR skeleton.
///
/// Parsing and I/O intentionally live outside this pure semantic boundary.
///
/// # Errors
///
/// Returns stable diagnostics when source-level semantic invariants fail.
pub fn compile(source: &Located<AppSource>) -> Result<AppIr, Vec<Diagnostic>> {
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
        }
    }

    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    Ok(AppIr {
        app_name: source.value.name.clone(),
        spec_version: source.value.spec_version,
        modules: source
            .value
            .modules
            .iter()
            .map(|module| ModuleIr {
                id: module.value.id.value.clone(),
                pages: module
                    .value
                    .pages
                    .iter()
                    .map(|page| lower_page(&page.value))
                    .collect(),
            })
            .collect(),
    })
}

fn lower_page(page: &PageSource) -> PageIr {
    PageIr {
        id: page.id.value.clone(),
        route: page.route.value.clone(),
        components: page
            .components
            .iter()
            .map(|component| lower_component(&component.value))
            .collect(),
    }
}

fn lower_component(component: &ComponentSource) -> ComponentIr {
    match component {
        ComponentSource::Text { value } => ComponentIr::Text {
            value: value.clone(),
        },
        ComponentSource::Field { label, binding } => ComponentIr::Field {
            label: label.clone(),
            binding: binding.clone(),
        },
        ComponentSource::Table { binding } => ComponentIr::Table {
            binding: binding.clone(),
        },
    }
}

#[cfg(test)]
mod tests {
    use uiko_core::{Located, ModuleId, SourceId, TextSpan};
    use uiko_source::{AppSource, ComponentSource, ModuleSource, PageSource};

    use super::{compile, SUPPORTED_SPEC_VERSION};

    fn at<T>(value: T, source: &str) -> Located<T> {
        Located::new(value, TextSpan::new(SourceId::new(source), 0, 10))
    }

    fn app_with_module(id: &str) -> Located<AppSource> {
        at(
            AppSource {
                name: "support-console".into(),
                spec_version: SUPPORTED_SPEC_VERSION,
                modules: vec![at(
                    ModuleSource {
                        id: at(ModuleId::new(id), "features/customers/module.jsonc"),
                        pages: vec![at(
                            PageSource {
                                id: at(
                                    "CustomerList".into(),
                                    "features/customers/list.jsonc",
                                ),
                                route: at(
                                    "/customers".into(),
                                    "features/customers/list.jsonc",
                                ),
                                components: vec![at(
                                    ComponentSource::Text {
                                        value: "Customers".into(),
                                    },
                                    "features/customers/list.jsonc",
                                )],
                            },
                            "features/customers/list.jsonc",
                        )],
                    },
                    "features/customers/module.jsonc",
                )],
            },
            "uiko.jsonc",
        )
    }

    #[test]
    fn identical_input_produces_identical_ir() {
        let app = app_with_module("customers");
        assert_eq!(
            compile(&app).expect("valid source"),
            compile(&app).expect("valid source")
        );
    }

    #[test]
    fn unknown_spec_version_fails_closed_with_stable_code() {
        let mut app = app_with_module("customers");
        app.value.spec_version = 999;
        assert_eq!(compile(&app).unwrap_err()[0].code, "UIKO1001");
    }

    #[test]
    fn duplicate_module_fails_before_lowering() {
        let mut app = app_with_module("customers");
        app.value.modules.push(app.value.modules[0].clone());
        assert!(
            compile(&app)
                .unwrap_err()
                .iter()
                .any(|diagnostic| diagnostic.code == "UIKO1101")
        );
    }

    #[test]
    fn duplicate_page_fails_before_lowering() {
        let mut app = app_with_module("customers");
        let page = app.value.modules[0].value.pages[0].clone();
        app.value.modules[0].value.pages.push(page);
        assert!(
            compile(&app)
                .unwrap_err()
                .iter()
                .any(|diagnostic| diagnostic.code == "UIKO1102")
        );
    }
}
