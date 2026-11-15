use super::types::GitWorktreeOption;
use iced::{Background, Color, Theme};
fn worktree(directory: &str, label: &str, branch: Option<&str>) -> GitWorktreeOption {
    GitWorktreeOption {
        directory: directory.to_string(),
        label: label.to_string(),
        branch: branch.map(str::to_string),
    }
}

fn background_color(background: &Option<Background>) -> Color {
    let Some(Background::Color(color)) = background else {
        panic!("expected color background");
    };
    *color
}

#[test]
fn task_718_test_module_is_wired() {
    let path = std::path::Path::new(file!());
    assert!(path.ends_with("header_tests.rs"));
}

#[test]
fn truncate_label_preserves_short_values_and_truncates_by_character() {
    assert_eq!(super::header::truncate_label("feature", 7), "feature");
    assert_eq!(super::header::truncate_label("features", 7), "featur…");
    assert_eq!(super::header::truncate_label("工作空间名字", 4), "工作空…");
}

#[test]
fn truncate_label_uses_ellipsis_at_zero_and_one_character_limits() {
    assert_eq!(super::header::truncate_label("a", 0), "…");
    assert_eq!(super::header::truncate_label("ab", 1), "…");
}

#[test]
fn truncate_label_preserves_empty_value_at_zero_limit() {
    assert_eq!(super::header::truncate_label("", 0), "");
}

#[test]
fn trigger_label_uses_trimmed_primary_segment_before_truncation() {
    assert_eq!(super::header::trigger_label("  primary · metadata "), "primary");
    assert_eq!(super::header::trigger_label("secondary · metadata"), "second…");
    assert_eq!(super::header::trigger_label("  alpha  "), "alpha");
}

#[test]
fn worktree_option_display_label_combines_primary_and_trimmed_branch() {
    let option =
        worktree("/repo/alpha", " alpha · ignored metadata ", Some(" feature/long-branch "));

    assert_eq!(super::header::worktree_option_display_label(&option), "alpha · feature/l…");
}

#[test]
fn worktree_option_display_label_omits_missing_or_blank_branch() {
    let without_branch = worktree("/repo/alpha", " alpha · ignored metadata ", None);
    let blank_branch = worktree("/repo/beta", " beta · ignored metadata ", Some("   "));

    assert_eq!(super::header::worktree_option_display_label(&without_branch), "alpha");
    assert_eq!(super::header::worktree_option_display_label(&blank_branch), "beta");
}

#[test]
fn worktree_button_style_uses_theme_background_for_each_state() {
    let theme = Theme::Dark;
    let palette = theme.extended_palette();
    let active = super::header::worktree_button_style(&theme, iced::widget::button::Status::Active);
    let hovered =
        super::header::worktree_button_style(&theme, iced::widget::button::Status::Hovered);
    let pressed =
        super::header::worktree_button_style(&theme, iced::widget::button::Status::Pressed);
    let disabled =
        super::header::worktree_button_style(&theme, iced::widget::button::Status::Disabled);

    assert_eq!(background_color(&active.background), palette.background.base.color);
    assert_eq!(background_color(&hovered.background), palette.background.weak.color);
    assert_eq!(background_color(&pressed.background), palette.background.strong.color);
    assert_eq!(background_color(&disabled.background), palette.background.base.color);
    assert_eq!(active.text_color, theme.palette().text);
    assert_eq!(active.border.width, 1.0);
    assert_eq!(active.border.color, palette.background.strong.color.scale_alpha(0.55));
    assert_eq!(active.border.radius, 10.0.into());
}
