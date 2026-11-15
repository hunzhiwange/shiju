use vw_api_app_provider_edit::{catalog::ProviderCatalog,presentation::AppProviderPresentationEntry};
#[derive(Clone, Debug)]
pub struct Snapshot {
    pub catalog: ProviderCatalog,
    pub presentation: Vec<AppProviderPresentationEntry>,
}

#[derive(Clone, Debug)]
pub struct Editor {
    pub id: String,
    pub sort: String,
    pub category: String,
    pub notes: String,
    pub saving: bool,
}
