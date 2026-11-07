use vw_gateway_client::vw_api_types::{
    agent_apps::CustomIconEntry,
    app_providers::icons::{AppProviderIcon, AppProviderIconChange},
};

#[derive(Clone)]
pub(crate) enum Draft {
    Upload { name: String, data: String },
    Rename { id: String, name: String },
    Delete { id: String, name: String },
}

#[derive(Clone, Default)]
pub(crate) struct Form {
    pub id: String,
    pub name: String,
    pub rows: Option<Vec<CustomIconEntry>>,
    pub current: Option<AppProviderIcon>,
    pub selection: Option<AppProviderIconChange>,
    pub draft: Option<Draft>,
    pub busy: bool,
    pub notice: Option<String>,
}

impl std::fmt::Debug for Form {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("IconForm([redacted])")
    }
}
