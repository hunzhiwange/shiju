//! Compatibility exports for application state composition.

pub use desktop_navigation::components::notification::Notification;
pub use desktop_ui_controls::toast::{Toast, ToastKind};
pub use desktop_web_runtime::state::WorkBrowserState;
pub use desktop_apps_ui::bookmark::{CookieConfig, WebBookmark};

pub use desktop_workspace::project_message::edit_meta::{RecentProjectMeta, default_recent_project_session_auto_refresh, default_recent_project_session_refresh_interval_seconds};

#[cfg(test)]
#[path = "app_aux_tests.rs"]
mod app_aux_tests;
