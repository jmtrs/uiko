#![forbid(unsafe_code)]

use uiko_core::ModuleId;

/// Authoring-neutral source DTO. JSONC is one front-end that will lower into this model.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppSource {
    pub name: String,
    pub spec_version: u32,
    pub modules: Vec<ModuleSource>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleSource {
    pub id: ModuleId,
    pub components: Vec<ComponentSource>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ComponentSource {
    Text { value: String },
    Field { label: String, binding: String },
    Table { binding: String },
}
