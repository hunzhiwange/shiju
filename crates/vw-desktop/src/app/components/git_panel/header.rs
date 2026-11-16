use crate::app::{App, Message};
use desktop_git_ui::git_panel::header as domain;
fn input(app: &App) -> domain::Input<'_> {
    domain::Input {
        desktop_locale: app.desktop_locale,
        selected_branch: &app.selected_branch,
        git_worktree_options: &app.git_worktree_options,
        selected_git_worktree_directory: &app.selected_git_worktree_directory,
        git_worktree_menu_open: app.git_worktree_menu_open,
        show_git_diff_summary: app.show_git_diff_summary,
        show_git_filter_options: app.show_git_filter_options,
        git_diff_fullscreen: app.git_diff_fullscreen,
        code_mode: app.workspace_mode == crate::app::state::WorkspaceMode::Code
    }
}
pub fn view(app: &App, files: Vec<String>) -> iced::Element<'_, Message> { domain::view(&input(app), files).map(super::view_messages::map) }
#[cfg(test)]
pub(super) fn worktree_menu_toggle_message(open: bool) -> Message { super::view_messages::map(domain::worktree_menu_toggle_message(open)) }
#[cfg(test)]
pub(super) fn filter_options_toggle_message(open: bool) -> Message { super::view_messages::map(domain::filter_options_toggle_message(open)) }
