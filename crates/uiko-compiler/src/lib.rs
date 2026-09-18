#![forbid(unsafe_code)]

use std::collections::BTreeSet;

use uiko_core::{AppIr, ComponentIr, Located, ModuleIr, TextSpan};
use uiko_source::{AppSource, ComponentSource};

pub const SUPPORTED_SPEC_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub code: &'static str,
    pub severity: Severity,
    pub message: String,
    pub span: TextSpan,
}

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
        if !module_ids.insert(module.id.clone()) {
            diagnostics.push(Diagnostic {
                code: "UIKO1101",
                severity: Severity::Error,
                message: format!("duplicate module `{}`", module.id),
                span: source.span.clone(),
            });
        }
    }

    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    let modules = source
        .value
        .modules
        .iter()
        .map(|module| ModuleIr {
            id: module.id.clone(),
            components: module.components.iter().map(lower_component).collect(),
        })
        .collect();

    Ok(AppIr {
        app_name: source.value.name.clone(),
        spec_version: source.value.spec_version,
        modules,
    })
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
    use uiko_source::{AppSource, ComponentSource, ModuleSource};

    use super::{compile, SUPPORTED_SPEC_VERSION};

    fn located(app: AppSource) -> Located<AppSource> {
        Located::new(app, TextSpan::new(SourceId::new("uiko.jsonc"), 0, 10))
    }

    #[test]
    fn identical_input_produces_identical_ir() {
        let app = located(AppSource {
            name: "support-console".into(),
            spec_version: SUPPORTED_SPEC_VERSION,
            modules: vec![ModuleSource {
                id: ModuleId::new("customers"),
                components: vec![ComponentSource::Text {
                    value: "Customers".into(),
                }],
            }],
        });

        let first = compile(&app).expect("valid source should compile");
        let second = compile(&app).expect("valid source should compile");
        assert_eq!(first, second);
    }

    #[test]
    fn unknown_spec_version_fails_closed_with_stable_code() {
        let app = located(AppSource {
            name: "support-console".into(),
            spec_version: 999,
            modules: vec![],
        });

        let diagnostics = compile(&app).expect_err("unsupported version must fail");
        assert_eq!(diagnostics[0].code, "UIKO1001");
    }

    #[test]
    fn duplicate_module_fails_before_lowering() {
        let module = ModuleSource {
            id: ModuleId::new("customers"),
            components: vec![],
        };
        let app = located(AppSource {
            name: "support-console".into(),
            spec_version: SUPPORTED_SPEC_VERSION,
            modules: vec![module.clone(), module],
        });

        let diagnostics = compile(&app).expect_err("duplicate module must fail");
        assert_eq!(diagnostics[0].code, "UIKO1101");
    }
}
