//! 桌面项目记录与代码审查视图的适配。
use crate::app::{App, Message, message};
use iced::Element;
pub fn view(app: &App) -> Element<'_, Message> {
    let (files, patches) = git_diff_by_file(app);
    build_code_review_view_from_diff(app, files, patches)
}
pub(super) fn build_code_review_view_from_diff(app: &App, files: Vec<String>, patches: std::collections::HashMap<String, String>) -> Element<'_, Message> {
    desktop_git_ui::code_review::build_code_review_view_from_diff(app.desktop_locale, &app.git.expanded_files_set, files, patches).map(|event| match event {
        desktop_git_ui::code_review::ReviewMessage::Refresh => Message::Git(message::GitMessage::RefreshGitPanelData),
        desktop_git_ui::code_review::ReviewMessage::ToggleExpandFile(file) => Message::Git(message::GitMessage::ToggleExpandFile(file)),
    })
}
pub(super) fn git_diff_by_file(
    app: &App,
) -> (Vec<String>, std::collections::HashMap<String, String>) {
    let Some(path) = &app.project_path else {
        return (vec![], std::collections::HashMap::new());
    };
    let records = crate::app::session_gateway::gateway_project_change_records(path);
    let mut files = Vec::with_capacity(records.len());
    let mut patches = std::collections::HashMap::<String, String>::with_capacity(records.len());

    for record in records {
        files.push(record.path.clone());
        patches.insert(record.path, record.patch);
    }

    (files, patches)
}
