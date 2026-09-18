#![forbid(unsafe_code)]

use uiko_core::{Located, ModuleId, ScalarValue};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppConfigSource {
    pub name: Located<String>,
    pub spec_version: Located<u32>,
    pub modules: Vec<Located<String>>,
    pub integrations: Vec<Located<String>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleConfigSource {
    pub id: Located<String>,
    pub pages: Vec<Located<String>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntegrationConfigSource {
    pub id: Located<String>,
    pub adapter: Located<String>,
    pub contract: Located<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppSource {
    pub name: String,
    pub spec_version: u32,
    pub modules: Vec<Located<ModuleSource>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleSource {
    pub id: Located<ModuleId>,
    pub pages: Vec<Located<PageSource>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageSource {
    pub id: Located<String>,
    pub route: Located<String>,
    pub state: Vec<Located<StateSource>>,
    pub queries: Vec<Located<QuerySource>>,
    pub components: Vec<Located<ComponentSource>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateSource {
    pub id: Located<String>,
    pub initial: Located<ScalarValue>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuerySource {
    pub id: Located<String>,
    pub operation: Located<String>,
    pub input: Vec<Located<InputBindingSource>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InputBindingSource {
    pub name: Located<String>,
    pub expression: Located<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentSource {
    pub id: Located<String>,
    pub kind: ComponentKindSource,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ComponentKindSource {
    Text {
        value: String,
    },
    Field {
        label: String,
        binding: String,
        fallback: Option<String>,
    },
    Table {
        binding: String,
        columns: Vec<TableColumnSource>,
    },
    Select {
        label: String,
        state: String,
        options: Vec<SelectOptionSource>,
    },
    Pagination {
        page_state: String,
        page_binding: String,
        page_size_binding: String,
        total_binding: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TableColumnSource {
    pub label: String,
    pub binding: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectOptionSource {
    pub label: String,
    pub value: ScalarValue,
}
