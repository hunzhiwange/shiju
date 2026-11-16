pub use desktop_chat_view::utils::time::{format_chat_time_label, relative_time_bucket, relative_time_label, relative_time_label_for_bucket};
use crate::app::App;
pub fn project_last_modified_ms(app: &App) -> Option<u64> { desktop_chat_view::utils::time::project_last_modified_ms(app.project_updated_at_ms, app.navigation_tabs.project_path.as_deref()) }
pub fn relative_modified_label(app: &App) -> String { desktop_chat_view::utils::time::relative_modified_label(crate::app::i18n::I18n::new(app.desktop_locale), project_last_modified_ms(app), crate::app::time::now_ms()) }
