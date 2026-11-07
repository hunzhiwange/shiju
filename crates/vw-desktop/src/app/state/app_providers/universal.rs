use vw_gateway_client::vw_api_types::{
    agent_apps::AppId,
    app_providers::{universal::UniversalProviderSummary, universal_sync::UniversalSyncResult},
};

#[derive(Clone, Debug, Default)]
pub(crate) struct Form {
    pub records: Option<Vec<UniversalProviderSummary>>,
    pub selected: Option<String>,
    pub apps: Vec<AppId>,
    pub results: Vec<UniversalSyncResult>,
    pub busy: bool,
    pub error: Option<String>,
    pub refresh_catalog_on_close: bool,
}

impl Form {
    pub fn selected_record(&self) -> Option<&UniversalProviderSummary> {
        self.records
            .as_ref()?
            .iter()
            .find(|record| Some(record.id.as_str()) == self.selected.as_deref())
    }
}
