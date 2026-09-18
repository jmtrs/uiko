#![forbid(unsafe_code)]

use uiko_core::{Located, ModuleId};

/// Root project source before module paths are loaded and resolved.
///
/// Authoring adapters lower their syntax into this DTO. It intentionally
/// contains no JSONC-specific types.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppConfigSource {
    pub name: Located<String>,
    pub spec_version: Located<u32>,
    pub modules: Vec<Located<String>>,
}

/// Authoring-neutral assembled source model consumed by semantic compilation.
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
