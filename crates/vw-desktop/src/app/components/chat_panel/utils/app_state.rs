use crate::app::App;
use desktop_chat_view::utils::app_state as state;
pub fn get_session_title(app: &App) -> String { state::get_session_title(app.active_session_id.as_deref(), app.sessions.iter().chain(app.project_sessions.values().flat_map(|sessions| sessions.iter())), crate::app::i18n::I18n::new(app.desktop_locale), |id, title| crate::app::i18n::other::session_title(app, id, title)) }
pub fn current_project_path_label(app: &App) -> String { state::current_project_path_label(app.project_path.as_deref(), crate::app::i18n::I18n::new(app.desktop_locale)) }
pub fn current_branch_label(app: &App) -> String { state::current_branch_label(app.git.selected_branch.as_deref()) }
pub fn is_recent_copy(app: &App, hash: u64) -> bool { state::is_recent_copy(app.last_copied_code_hash, app.last_copy_time, hash) }
