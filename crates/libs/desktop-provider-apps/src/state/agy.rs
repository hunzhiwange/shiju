use vw_api_app_provider_profiles::agy::AgyFormView;
#[derive(Clone, Default)]
pub struct Draft {
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
