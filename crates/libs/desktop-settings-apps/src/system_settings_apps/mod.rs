pub mod catalog;
pub mod local;
pub mod launch;
pub mod launch_view;
pub mod preferences;
pub mod preferences_view;
pub mod lifecycle;
pub mod drag;
pub mod presentation;
mod styles;
pub mod list;
pub mod details;
pub mod domains;
use vw_api_app_identity::{AppId, AppInstallationId};
use vw_api_app_operations::AppActionPreviewRequest;
#[derive(Clone, Debug)]
pub enum Action {
    Refresh, QueryInput(String), ConnectionOpen, LocalConnect,
    ToggleDetails(AppId), RefreshObservation(AppId),
    PreferencesOpen, LaunchOpen(AppId, AppInstallationId),
    Preview(AppActionPreviewRequest), LifecycleClose, LifecycleConfirm, LifecycleRefresh, LifecycleCancel,
    Drag(drag::DragMessage), Providers(AppId), Prompts(AppId), Download,
}
pub struct ViewState<'a> {
    pub rows: &'a [catalog::CatalogRow], pub preferences: &'a preferences::PreferencesState,
    pub lifecycle: &'a lifecycle::LifecycleState, pub drag: &'a drag::DragState,
    pub query: &'a str, pub host: &'a str, pub loading: bool,
    pub error: &'a Option<String>, pub expanded: Option<AppId>, pub hovered: Option<AppId>,
}
pub mod lifecycle_view;
#[cfg(test)]
mod catalog_tests;
#[cfg(test)]
mod launch_tests;
#[cfg(test)]
mod preferences_tests;
#[cfg(test)]
mod lifecycle_tests;
#[cfg(test)]
mod drag_tests;
#[cfg(test)]
mod presentation_tests;
pub mod uninstall;
