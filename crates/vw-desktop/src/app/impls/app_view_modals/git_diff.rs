use super::{App, Message};
use iced::Element;
#[allow(dead_code)]
pub(crate) fn with_git_diff_comment<'a>(app: &'a App, root: Element<'a, Message>) -> Element<'a, Message> {
    desktop_git_ui::modal_overlays::with_git_diff_comment(crate::app::components::git_panel::diff_comment_modal(app), root)
}
pub(crate) fn with_git_diff_overlays<'a>(app: &'a App, root: Element<'a, Message>) -> Element<'a, Message> {
    desktop_git_ui::modal_overlays::with_git_diff_overlays(crate::app::components::git_panel::diff_comment_modal(app), crate::app::components::git_panel::discard_file_modal(app), root)
}
