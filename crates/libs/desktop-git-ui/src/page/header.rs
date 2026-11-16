use crate::git_panel::header as domain;
use crate::page::Message;
use crate::page::ViewContext;
fn input<'a>(app: &ViewContext<'a>) -> domain::Input<'a> {
    domain::Input {
        desktop_locale: app.desktop_locale,
        selected_branch: &app.git.selected_branch,
        git_worktree_options: &app.git.git_worktree_options,
        selected_git_worktree_directory: &app.git.selected_git_worktree_directory,
        git_worktree_menu_open: app.git.git_worktree_menu_open,
        show_git_diff_summary: app.git.show_git_diff_summary,
        show_git_filter_options: app.git.show_git_filter_options,
        git_diff_fullscreen: app.git.git_diff_fullscreen,
        code_mode: app.code_mode,
    }
}
pub fn view<'a>(app: &ViewContext<'a>, files: Vec<String>) -> iced::Element<'a, Message> {
    domain::view(&input(app), files).map(super::view_messages::map)
}
#[cfg(test)]
pub(super) fn worktree_menu_toggle_message(open: bool) -> Message {
    super::view_messages::map(domain::worktree_menu_toggle_message(open))
}
#[cfg(test)]
pub(super) fn filter_options_toggle_message(open: bool) -> Message {
    super::view_messages::map(domain::filter_options_toggle_message(open))
}
