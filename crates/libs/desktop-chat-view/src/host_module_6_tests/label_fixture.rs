#[derive(Default)]
pub struct LabelFixture {
    pub active_session_id: Option<String>,
    pub project_path: Option<String>,
    pub selected_branch: Option<String>,
    pub sessions: Vec<session_info::Info>,
    pub project_sessions: std::collections::HashMap<String, Vec<session_info::Info>>,
}
impl LabelFixture {
    pub fn new() -> (Self, ()) {
        (Self::default(), ())
    }
}
fn i18n() -> desktop_i18n::I18n {
    desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese)
}
pub fn current_project_path_label(app: &LabelFixture) -> String {
    crate::utils::app_state::current_project_path_label(app.project_path.as_deref(), i18n())
}
pub fn current_branch_label(app: &LabelFixture) -> String {
    crate::utils::app_state::current_branch_label(app.selected_branch.as_deref())
}
pub fn get_session_title(app: &LabelFixture) -> String {
    crate::utils::app_state::get_session_title(
        app.active_session_id.as_deref(),
        app.sessions.iter().chain(app.project_sessions.values().flatten()),
        i18n(),
        |_, title| {
            if title.trim().is_empty() {
                i18n().__("新会话").to_owned()
            } else {
                title.to_owned()
            }
        },
    )
}
