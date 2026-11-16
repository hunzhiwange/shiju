use crate::app::App;
use desktop_chat_view::utils::app_state as state;
pub fn get_session_title(app: &App) -> String { state::get_session_title(app.session_state.active_session_id.as_deref(), app.session_state.sessions.iter().chain(app.workspace_state.project_sessions.values().flat_map(|sessions| sessions.iter())), crate::app::i18n::I18n::new(app.general_settings.desktop_locale), |id, title| crate::app::i18n::other::session_title(app, id, title)) }
pub fn current_project_path_label(app: &App) -> String { state::current_project_path_label(app.navigation_tabs.project_path.as_deref(), crate::app::i18n::I18n::new(app.general_settings.desktop_locale)) }
pub fn current_branch_label(app: &App) -> String { state::current_branch_label(app.git.selected_branch.as_deref()) }
pub fn is_recent_copy(app: &App, hash: u64) -> bool { state::is_recent_copy(app.clipboard_state.last_copied_code_hash, app.clipboard_state.last_copy_time, hash) }
