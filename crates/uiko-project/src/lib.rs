#![forbid(unsafe_code)]

use std::{
    collections::btree_map::Entry,
    fs,
    path::{Component, Path, PathBuf},
};

use uiko_capabilities::CapabilityCatalog;
use uiko_core::{Diagnostic, Located, ModuleId, SourceId, TextSpan};
use uiko_openapi::{OpenApiTransportCatalog, adapter_id, import_openapi_provider};
use uiko_source::{AppSource, ModuleSource};
use uiko_source_jsonc::{
    parse_app_config, parse_integration_config, parse_module_config, parse_page,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoadedProject {
    pub source: Located<AppSource>,
    pub capabilities: CapabilityCatalog,
    pub openapi_transport: OpenApiTransportCatalog,
}

/// Load one filesystem-backed uiko project plus its normalized capability contracts.
///
/// # Errors
///
/// Returns stable diagnostics for unreadable source, path confinement failures,
/// invalid integration declarations, unsupported contracts and JSONC source errors.
pub fn load_project(root: &Path) -> Result<LoadedProject, Vec<Diagnostic>> {
    let root = fs::canonicalize(root).map_err(|error| {
        vec![Diagnostic::error(
            "UIKO1200",
            format!("cannot open project root: {error}"),
            empty_span("uiko.jsonc"),
        )]
    })?;

    let app_path = root.join("uiko.jsonc");
    let app_text = fs::read_to_string(&app_path).map_err(|error| {
        vec![Diagnostic::error(
            "UIKO1201",
            format!("cannot read uiko.jsonc: {error}"),
            empty_span("uiko.jsonc"),
        )]
    })?;
    let app_config = parse_app_config(&source_id(&root, &app_path), &app_text)?;

    let mut modules = Vec::with_capacity(app_config.value.modules.len());
    let mut diagnostics = Vec::new();
    for module_ref in &app_config.value.modules {
        match load_module(&root, module_ref) {
            Ok(module) => modules.push(module),
            Err(errors) => diagnostics.extend(errors),
        }
    }

    let mut capabilities = CapabilityCatalog::default();
    let mut openapi_transport = OpenApiTransportCatalog::default();
    for integration_ref in &app_config.value.integrations {
        match load_integration(&root, integration_ref) {
            Ok(imported) => match capabilities
                .providers
                .entry(imported.capabilities.id.clone())
            {
                Entry::Vacant(entry) => {
                    openapi_transport
                        .providers
                        .insert(imported.transport.id.clone(), imported.transport);
                    entry.insert(imported.capabilities);
                }
                Entry::Occupied(_) => diagnostics.push(Diagnostic::error(
                    "UIKO1203",
                    format!("duplicate integration id `{}`", imported.capabilities.id),
                    integration_ref.span.clone(),
                )),
            },
            Err(errors) => diagnostics.extend(errors),
        }
    }

    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    let app_span = app_config.span;
    Ok(LoadedProject {
        source: Located::new(
            AppSource {
                name: app_config.value.name.value,
                spec_version: app_config.value.spec_version.value,
                modules,
            },
            app_span,
        ),
        capabilities,
        openapi_transport,
    })
}

fn load_integration(
    root: &Path,
    integration_ref: &Located<String>,
) -> Result<uiko_openapi::ImportedOpenApiProvider, Vec<Diagnostic>> {
    let integration_path = resolve_reference(
        root,
        root,
        &integration_ref.value,
        None,
        &integration_ref.span,
    )?;
    let text = read_referenced_source(root, &integration_path, &integration_ref.span)?;
    let config = parse_integration_config(&source_id(root, &integration_path), &text)?;

    if config.value.adapter.value != adapter_id() {
        return Err(vec![Diagnostic::error(
            "UIKO1204",
            format!(
                "unsupported integration adapter `{}`",
                config.value.adapter.value
            ),
            config.value.adapter.span,
        )]);
    }

    let integration_dir = integration_path
        .parent()
        .expect("resolved integration file always has a parent");
    let contract_path = resolve_reference(
        root,
        integration_dir,
        &config.value.contract.value,
        None,
        &config.value.contract.span,
    )?;
    let contract_text = read_referenced_source(root, &contract_path, &config.value.contract.span)?;
    let contract_source = source_id(root, &contract_path);

    import_openapi_provider(&config.value.id.value, &contract_source, &contract_text)
}

fn load_module(
    root: &Path,
    module_ref: &Located<String>,
) -> Result<Located<ModuleSource>, Vec<Diagnostic>> {
    let module_path = resolve_reference(
        root,
        root,
        &module_ref.value,
        Some("module.jsonc"),
        &module_ref.span,
    )?;
    let text = read_referenced_source(root, &module_path, &module_ref.span)?;
    let module_config = parse_module_config(&source_id(root, &module_path), &text)?;
    let module_dir = module_path
        .parent()
        .expect("resolved module file always has a parent");

    let mut pages = Vec::with_capacity(module_config.value.pages.len());
    let mut diagnostics = Vec::new();
    for page_ref in &module_config.value.pages {
        let page_path =
            match resolve_reference(root, module_dir, &page_ref.value, None, &page_ref.span) {
                Ok(path) => path,
                Err(errors) => {
                    diagnostics.extend(errors);
                    continue;
                }
            };
        let page_text = match read_referenced_source(root, &page_path, &page_ref.span) {
            Ok(text) => text,
            Err(errors) => {
                diagnostics.extend(errors);
                continue;
            }
        };
        match parse_page(&source_id(root, &page_path), &page_text) {
            Ok(page) => pages.push(page),
            Err(errors) => diagnostics.extend(errors),
        }
    }
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    let module_span = module_config.span;
    Ok(Located::new(
        ModuleSource {
            id: Located::new(
                ModuleId::new(module_config.value.id.value),
                module_config.value.id.span,
            ),
            pages,
        },
        module_span,
    ))
}

fn resolve_reference(
    root: &Path,
    base: &Path,
    raw: &str,
    leaf: Option<&str>,
    reference_span: &TextSpan,
) -> Result<PathBuf, Vec<Diagnostic>> {
    let relative = Path::new(raw);
    if relative.is_absolute()
        || relative.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(vec![Diagnostic::error(
            "UIKO1202",
            format!("source path escapes project root: {raw}"),
            reference_span.clone(),
        )]);
    }

    let mut candidate = base.join(relative);
    if let Some(leaf) = leaf {
        candidate.push(leaf);
    }

    let canonical = fs::canonicalize(&candidate).map_err(|error| {
        vec![Diagnostic::error(
            "UIKO1201",
            format!("cannot resolve source `{raw}`: {error}"),
            reference_span.clone(),
        )]
    })?;

    if !canonical.starts_with(root) {
        return Err(vec![Diagnostic::error(
            "UIKO1202",
            format!("source path escapes project root: {raw}"),
            reference_span.clone(),
        )]);
    }

    Ok(canonical)
}

fn read_referenced_source(
    root: &Path,
    path: &Path,
    reference_span: &TextSpan,
) -> Result<String, Vec<Diagnostic>> {
    fs::read_to_string(path).map_err(|error| {
        vec![Diagnostic::error(
            "UIKO1201",
            format!(
                "cannot read source `{}`: {error}",
                source_id(root, path).as_str()
            ),
            reference_span.clone(),
        )]
    })
}

fn source_id(root: &Path, path: &Path) -> SourceId {
    let relative = path.strip_prefix(root).unwrap_or(path);
    SourceId::new(relative.to_string_lossy().replace('\\', "/"))
}

fn empty_span(source: &str) -> TextSpan {
    TextSpan::new(SourceId::new(source), 0, 0)
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::Path,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::load_project;

    #[test]
    fn support_console_fixture_loads_contract_capabilities() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../experiments/fixtures/compiler-support-console");
        let project = load_project(&root).expect("fixture should load");
        assert_eq!(project.source.value.name, "support-console");
        assert_eq!(project.source.value.modules.len(), 1);
        assert!(
            project
                .capabilities
                .providers
                .get("crm")
                .expect("crm provider")
                .operations
                .contains_key("getCustomer")
        );
    }

    #[test]
    fn parent_directory_reference_is_rejected() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("uiko-path-test-{unique}"));
        fs::create_dir_all(&root).expect("temporary project directory");
        fs::write(
            root.join("uiko.jsonc"),
            r#"{ "name": "escape", "specVersion": 1, "modules": ["../outside"] }"#,
        )
        .expect("temporary project source");

        let diagnostics = load_project(&root).expect_err("path escape must fail");
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "UIKO1202")
        );

        fs::remove_dir_all(root).expect("temporary project cleanup");
    }
}
