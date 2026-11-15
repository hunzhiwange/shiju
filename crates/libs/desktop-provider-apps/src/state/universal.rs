use vw_api_types::agent_apps::AppId;
use vw_api_app_provider_edit::universal::UniversalProviderSummary;
use vw_api_app_provider_edit::universal_sync::UniversalSyncResult;

#[derive(Clone, Debug, Default)]
pub struct Form {
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
