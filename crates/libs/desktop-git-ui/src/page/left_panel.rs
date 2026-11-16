use crate::git_panel::left_panel as domain;
use crate::page::Message;
use crate::page::ViewContext;
pub(super) use domain::{COMMIT_HELP_TEXT, COMMIT_HELP_TITLE};
#[cfg(test)]
pub(super) use domain::{CommitButtonState, CommitPanelState};
fn input<'a>(app: &ViewContext<'a>) -> domain::Input<'a> {
    domain::Input {
        desktop_locale: app.desktop_locale,
        show_git_diff_summary: app.git.show_git_diff_summary,
        git_commit_type: app.git.git_commit_type,
        git_commit_scope: &app.git.git_commit_scope,
        git_commit_message: &app.git.git_commit_message,
        git_commit_description_editor: &app.git.git_commit_description_editor,
        git_commit_in_progress: app.git.git_commit_in_progress,
        file_manager_refresh_frame: app.refresh_frame,
        any_selected: has_selected_changes(app),
    }
}
pub(super) fn has_selected_changes(app: &ViewContext) -> bool {
    !app.git.staged_files_selected.is_empty()
        || !app.git.staged_hunks_selected.is_empty()
        || !app.git.staged_lines_selected.is_empty()
        || !app.git.staged_old_lines_selected.is_empty()
}
#[cfg(test)]
pub(super) fn commit_panel_state(app: &ViewContext) -> CommitPanelState {
    domain::commit_panel_state(&input(app))
}
pub fn view<'a>(app: &ViewContext<'a>) -> iced::Element<'a, Message> {
    let i18n = desktop_i18n::I18n::new(app.desktop_locale);
    let explanation = domain::mode_description(app.git.show_git_diff_summary);
    let title = i18n.__("提交所选变更");
    let help = crate::page::help::settings_hover_help_label(
        i18n,
        None,
        title,
        iced::widget::text(title).size(14),
        i18n.__(explanation),
    );
    domain::view(
        &input(app),
        iced::Element::from(help).map(crate::git_panel::messages::UiMessage::External),
    )
    .map(super::view_messages::map)
}

#[cfg(test)]
pub(crate) use domain::traverse_commit_focus;
