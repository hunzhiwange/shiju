use vw_gateway_client::vw_api_types::app_providers::agy::AgyFormView;
#[derive(Clone, Default)]
pub(crate) struct Draft {
    pub model: String,
    pub base_url: String,
    pub has_saved_key: bool,
    pub revision: Option<String>,
}
impl Draft {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn from_view(view: &AgyFormView) -> Self {
        Self {
            model: view.model.clone().unwrap_or_default(),
            base_url: view.base_url.clone().unwrap_or_default(),
            has_saved_key: view.has_saved_key,
            revision: Some(view.revision.clone()),
        }
    }
}
