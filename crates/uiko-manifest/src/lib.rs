#![forbid(unsafe_code)]

use std::fmt::Write as _;

use uiko_core::{AppIr, ComponentKindIr};

pub const UI_MANIFEST_SPEC_VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiManifest {
    pub spec_version: u32,
    pub app_name: String,
    pub routes: Vec<UiRouteManifest>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiRouteManifest {
    pub id: String,
    pub path: String,
    pub components: Vec<UiComponentManifest>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiComponentManifest {
    pub id: String,
    pub kind: UiComponentKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UiComponentKind {
    Text { value: String },
    Field { label: String, binding: String },
    Table { binding: String },
}

#[must_use]
pub fn derive_ui_manifest(ir: &AppIr) -> UiManifest {
    UiManifest {
        spec_version: UI_MANIFEST_SPEC_VERSION,
        app_name: ir.app_name.clone(),
        routes: ir
            .modules
            .iter()
            .flat_map(|module| {
                module.pages.iter().map(|page| {
                    let route_id = format!("{}.{}", module.id.as_str(), page.id);
                    UiRouteManifest {
                        id: route_id.clone(),
                        path: page.route.clone(),
                        components: page
                            .components
                            .iter()
                            .map(|component| UiComponentManifest {
                                id: format!("{route_id}.{}", component.id),
                                kind: match &component.kind {
                                    ComponentKindIr::Text { value } => UiComponentKind::Text {
                                        value: value.clone(),
                                    },
                                    ComponentKindIr::Field { label, binding } => {
                                        UiComponentKind::Field {
                                            label: label.clone(),
                                            binding: binding.clone(),
                                        }
                                    }
                                    ComponentKindIr::Table { binding } => UiComponentKind::Table {
                                        binding: binding.clone(),
                                    },
                                },
                            })
                            .collect(),
                    }
                })
            })
            .collect(),
    }
}

impl UiManifest {
    #[must_use]
    pub fn to_json_pretty(&self) -> String {
        let mut output = String::new();
        writeln!(output, "{{").expect("writing to String cannot fail");
        writeln!(output, "  \"specVersion\": {},", self.spec_version)
            .expect("writing to String cannot fail");
        writeln!(output, "  \"app\": \"{}\",", json_escape(&self.app_name))
            .expect("writing to String cannot fail");
        writeln!(output, "  \"routes\": [").expect("writing to String cannot fail");

        for (route_index, route) in self.routes.iter().enumerate() {
            writeln!(output, "    {{").expect("writing to String cannot fail");
            writeln!(output, "      \"id\": \"{}\",", json_escape(&route.id))
                .expect("writing to String cannot fail");
            writeln!(output, "      \"path\": \"{}\",", json_escape(&route.path))
                .expect("writing to String cannot fail");
            writeln!(output, "      \"components\": [").expect("writing to String cannot fail");

            for (component_index, component) in route.components.iter().enumerate() {
                write_component(&mut output, component, 8);
                if component_index + 1 < route.components.len() {
                    output.push(',');
                }
                output.push('\n');
            }

            writeln!(output, "      ]").expect("writing to String cannot fail");
            write!(output, "    }}").expect("writing to String cannot fail");
            if route_index + 1 < self.routes.len() {
                output.push(',');
            }
            output.push('\n');
        }

        writeln!(output, "  ]").expect("writing to String cannot fail");
        output.push('}');
        output
    }
}

fn write_component(output: &mut String, component: &UiComponentManifest, indent: usize) {
    let pad = " ".repeat(indent);
    write!(output, "{pad}{{\"id\":\"{}\",", json_escape(&component.id))
        .expect("writing to String cannot fail");

    match &component.kind {
        UiComponentKind::Text { value } => {
            write!(
                output,
                "\"kind\":\"Text\",\"value\":\"{}\"}}",
                json_escape(value)
            )
            .expect("writing to String cannot fail");
        }
        UiComponentKind::Field { label, binding } => {
            write!(
                output,
                "\"kind\":\"Field\",\"label\":\"{}\",\"binding\":\"{}\"}}",
                json_escape(label),
                json_escape(binding)
            )
            .expect("writing to String cannot fail");
        }
        UiComponentKind::Table { binding } => {
            write!(
                output,
                "\"kind\":\"Table\",\"binding\":\"{}\"}}",
                json_escape(binding)
            )
            .expect("writing to String cannot fail");
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

#[cfg(test)]
mod tests {
    use uiko_core::{AppIr, ComponentIr, ComponentKindIr, ModuleId, ModuleIr, PageIr};

    use super::{UI_MANIFEST_SPEC_VERSION, derive_ui_manifest};

    fn fixture_ir() -> AppIr {
        AppIr {
            app_name: "support-console".into(),
            spec_version: 1,
            modules: vec![ModuleIr {
                id: ModuleId::new("customers"),
                pages: vec![PageIr {
                    id: "CustomerDetail".into(),
                    route: "/customers/:customerId".into(),
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
                            },
                        },
                    ],
                }],
            }],
        }
    }

    #[test]
    fn public_ids_derive_from_symbolic_identity_not_position() {
        let manifest = derive_ui_manifest(&fixture_ir());

        assert_eq!(manifest.spec_version, UI_MANIFEST_SPEC_VERSION);
        assert_eq!(manifest.routes[0].id, "customers.CustomerDetail");
        assert_eq!(
            manifest.routes[0].components[0].id,
            "customers.CustomerDetail.title"
        );
        assert_eq!(
            manifest.routes[0].components[1].id,
            "customers.CustomerDetail.name"
        );
    }

    #[test]
    fn manifest_json_is_deterministic_and_escapes_text() {
        let mut ir = fixture_ir();
        ir.modules[0].pages[0].components[0].kind = ComponentKindIr::Text {
            value: "Customer \"A\"".into(),
        };

        let first = derive_ui_manifest(&ir).to_json_pretty();
        let second = derive_ui_manifest(&ir).to_json_pretty();

        assert_eq!(first, second);
        assert!(first.contains(r#"\"value\":\"Customer \\\"A\\\"\""#));
    }
}
