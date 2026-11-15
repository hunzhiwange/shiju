use crate::app::{App, Message};
use desktop_git_ui::git_panel::left_panel as domain;
pub(super) use domain::{COMMIT_HELP_TITLE, COMMIT_HELP_TEXT};
#[cfg(test)]
pub(super) use domain::{CommitButtonState, CommitPanelState};
fn input(app: &App) -> domain::Input<'_> {
    domain::Input {
        desktop_locale: app.desktop_locale,
        show_git_diff_summary: app.show_git_diff_summary,
        git_commit_type: app.git_commit_type,
        git_commit_scope: &app.git_commit_scope,
        git_commit_message: &app.git_commit_message,
        git_commit_description_editor: &app.git_commit_description_editor,
        git_commit_in_progress: app.git_commit_in_progress,
        file_manager_refresh_frame: app.file_manager.file_manager_refresh_frame,
        any_selected: has_selected_changes(app)
    }
}
pub(super) fn has_selected_changes(app: &App) -> bool { !app.staged_files_selected.is_empty() || !app.staged_hunks_selected.is_empty() || !app.staged_lines_selected.is_empty() || !app.staged_old_lines_selected.is_empty() }
#[cfg(test)]
pub(super) fn commit_panel_state(app: &App) -> CommitPanelState { domain::commit_panel_state(&input(app)) }
pub fn view(app: &App) -> iced::Element<'_, Message> {
 let i18n = crate::app::i18n::I18n::new(app.desktop_locale);
 let explanation = domain::mode_description(app.show_git_diff_summary);
 let title = i18n.__("提交所选变更");
 let help = crate::app::components::system_settings_common::settings_hover_help_label(i18n, None, title, iced::widget::text(title).size(14), i18n.__(explanation));
 domain::view(&input(app), iced::Element::from(help).map(desktop_git_ui::git_panel::messages::UiMessage::External)).map(super::view_messages::map)
}

pub(crate) use domain::traverse_commit_focus;
