pub use desktop_chat_view::utils::path::{normalize_file_url_to_path, normalize_file_reference_to_path};
use crate::app::App;
pub fn resolve_path(app: &App, p: &str) -> Option<String> { desktop_chat_view::utils::path::resolve_path(app.navigation_tabs.project_path.as_deref(), p) }
pub fn relative_to_project_root(app: &App, p: &str) -> Option<String> { desktop_chat_view::utils::path::relative_to_project_root(app.navigation_tabs.project_path.as_deref(), p) }
