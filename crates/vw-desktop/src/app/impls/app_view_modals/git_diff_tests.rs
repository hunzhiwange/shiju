use super::*;
use crate::app::state::{GitDiffCommentDraft, GitDiffLineRange};
use iced::widget::text_editor;

fn test_app() -> App {
    App::new().0
}

fn root() -> iced::Element<'static, Message> {
    iced::widget::container(iced::widget::text("root")).into()
}

fn comment_draft() -> GitDiffCommentDraft {
    GitDiffCommentDraft {
        range: GitDiffLineRange { file: "src/lib.rs".to_string(), start: 2, end: 4, is_old: false },
        editor: text_editor::Content::with_text("Please clarify this change"),
    }
}

#[test]
fn with_git_diff_comment_returns_root_when_comment_modal_is_absent() {
    let app = test_app();

    let _element: iced::Element<'_, Message> = with_git_diff_comment(&app, root());
}

#[test]
fn with_git_diff_comment_layers_modal_when_comment_draft_exists() {
    let mut app = test_app();
    app.git_diff_comment_draft = Some(comment_draft());

    let _element: iced::Element<'_, Message> = with_git_diff_comment(&app, root());
}

#[test]
fn with_git_diff_overlays_builds_empty_overlay_layer_when_modals_are_absent() {
    let app = test_app();

    let _element: iced::Element<'_, Message> = with_git_diff_overlays(&app, root());
}

#[test]
fn with_git_diff_overlays_builds_comment_overlay_without_discard_modal() {
    let mut app = test_app();
    app.git_diff_comment_draft = Some(comment_draft());

    let _element: iced::Element<'_, Message> = with_git_diff_overlays(&app, root());
}

#[test]
fn with_git_diff_overlays_builds_discard_overlay_without_comment_modal() {
    let mut app = test_app();
    app.file_to_discard = Some("src/lib.rs".to_string());

    let _element: iced::Element<'_, Message> = with_git_diff_overlays(&app, root());
}

#[test]
fn with_git_diff_overlays_layers_comment_and_discard_modals_together() {
    let mut app = test_app();
    app.git_diff_comment_draft = Some(comment_draft());
    app.file_to_discard = Some("src/lib.rs".to_string());

    let _element: iced::Element<'_, Message> = with_git_diff_overlays(&app, root());
}
