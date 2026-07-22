use super::comment_editor::{
    diff_comment_cancel_button_style, diff_comment_editor, diff_comment_editor_action,
    diff_comment_editor_card_style, diff_comment_file_label, diff_comment_range_label,
    diff_comment_submit_button_style,
};
use crate::app::state::{GitDiffCommentDraft, GitDiffLineRange};
use crate::app::{App, Message, message};
use iced::widget::{button, text_editor};
use iced::{Background, Color, Theme, Vector};

fn app() -> App {
    App::new().0
}

#[test]
fn comment_editor_is_absent_without_draft() {
    let app = app();

    assert!(diff_comment_editor(&app).is_none());
}

#[test]
fn comment_editor_builds_for_empty_and_populated_drafts() {
    let mut app = app();
    app.git_diff_comment_draft = Some(GitDiffCommentDraft {
        range: GitDiffLineRange {
            file: "src/very_long_file_name_for_comment_editor.rs".to_string(),
            start: 1,
            end: 3,
            is_old: false,
        },
        editor: text_editor::Content::with_text("Looks good"),
    });
    assert!(diff_comment_editor(&app).is_some());

    app.git_diff_comment_draft = Some(GitDiffCommentDraft {
        range: GitDiffLineRange { file: "old.rs".to_string(), start: 4, end: 4, is_old: true },
        editor: text_editor::Content::new(),
    });
    assert!(diff_comment_editor(&app).is_some());
}

#[test]
fn comment_file_label_uses_basename_and_preserves_short_or_exact_limit_names() {
    assert_eq!(diff_comment_file_label("src/lib.rs"), "lib.rs");
    assert_eq!(
        diff_comment_file_label("src/1234567890123456789012345678"),
        "1234567890123456789012345678"
    );
}

#[test]
fn comment_file_label_truncates_long_ascii_and_unicode_names_by_character() {
    assert_eq!(
        diff_comment_file_label("src/1234567890123456789012345678.rs"),
        "1234567890123456789012345678…"
    );

    let unicode_name = format!("{}尾", "界".repeat(28));
    assert_eq!(diff_comment_file_label(&unicode_name), format!("{}…", "界".repeat(28)));
}

#[test]
fn comment_file_label_falls_back_when_path_has_no_file_name() {
    assert_eq!(diff_comment_file_label(""), "");
}

#[test]
fn comment_range_label_converts_zero_based_old_and_new_ranges() {
    assert_eq!(diff_comment_range_label(0, 0, false), "正在评论 新行 1");
    assert_eq!(diff_comment_range_label(4, 4, true), "正在评论 旧行 5");
    assert_eq!(diff_comment_range_label(1, 3, false), "正在评论 新行 2-4");
    assert_eq!(diff_comment_range_label(7, 9, true), "正在评论 旧行 8-10");
}

#[test]
fn comment_editor_action_wraps_the_original_text_edit() {
    let action = text_editor::Action::Edit(text_editor::Edit::Insert('好'));
    let message = diff_comment_editor_action(action.clone());

    let Message::Git(message::GitMessage::DiffCommentEditorAction(wrapped)) = message else {
        panic!("expected diff comment editor action");
    };
    assert_eq!(wrapped, action);
}

#[test]
fn cancel_button_style_covers_idle_hovered_pressed_and_disabled_statuses() {
    for theme in [Theme::Light, Theme::Dark] {
        let palette = theme.extended_palette();
        let active = diff_comment_cancel_button_style(&theme, button::Status::Active);
        let hovered = diff_comment_cancel_button_style(&theme, button::Status::Hovered);
        let pressed = diff_comment_cancel_button_style(&theme, button::Status::Pressed);
        let disabled = diff_comment_cancel_button_style(&theme, button::Status::Disabled);

        assert_eq!(background_color(active.background), palette.background.base.color);
        assert_eq!(background_color(hovered.background), palette.background.weak.color);
        assert_eq!(background_color(pressed.background), palette.background.strong.color);
        assert_eq!(background_color(disabled.background), palette.background.base.color);
        assert_eq!(active.text_color, theme.palette().text);
        assert_eq!(active.border.width, 1.0);
        assert_eq!(active.border.color, palette.background.strong.color);
        assert_eq!(active.border.radius, 10.0.into());
    }
}

#[test]
fn submit_button_style_covers_idle_hovered_pressed_and_disabled_statuses() {
    for theme in [Theme::Light, Theme::Dark] {
        let primary = theme.palette().primary;
        let active = diff_comment_submit_button_style(&theme, button::Status::Active);
        let hovered = diff_comment_submit_button_style(&theme, button::Status::Hovered);
        let pressed = diff_comment_submit_button_style(&theme, button::Status::Pressed);
        let disabled = diff_comment_submit_button_style(&theme, button::Status::Disabled);

        assert_eq!(background_color(active.background), primary);
        assert_eq!(background_color(hovered.background), primary.scale_alpha(0.92));
        assert_eq!(background_color(pressed.background), primary.scale_alpha(0.86));
        assert_eq!(background_color(disabled.background), primary);
        assert_eq!(active.text_color, Color::WHITE);
        assert_eq!(active.border.width, 0.0);
        assert_eq!(active.border.color, Color::TRANSPARENT);
        assert_eq!(active.border.radius, 10.0.into());
    }
}

#[test]
fn editor_card_style_uses_theme_surface_border_and_shadow() {
    for theme in [Theme::Light, Theme::Dark] {
        let palette = theme.extended_palette();
        let style = diff_comment_editor_card_style(&theme);

        assert_eq!(background_color(style.background), palette.background.base.color);
        assert_eq!(style.border.width, 1.0);
        assert_eq!(style.border.color, palette.background.strong.color);
        assert_eq!(style.border.radius, 12.0.into());
        assert_eq!(style.shadow.color, Color::BLACK.scale_alpha(0.25));
        assert_eq!(style.shadow.offset, Vector::new(0.0, 8.0));
        assert_eq!(style.shadow.blur_radius, 18.0);
    }
}

fn background_color(background: Option<Background>) -> Color {
    match background {
        Some(Background::Color(color)) => color,
        other => panic!("expected color background, got {other:?}"),
    }
}
