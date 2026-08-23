//! 验证 Git 面板标题栏的工作树标签、切换消息与视图组合。
//! 覆盖字符级截断、空分支名和未匹配工作树时的显示回退。

use iced::{Background, Color, Theme};

use crate::app::state::GitWorktreeOption;
use crate::app::{Message, message};

#[test]
fn task_718_test_module_is_wired() {
    let path = std::path::Path::new(file!());
    assert!(path.ends_with("header_tests.rs"));
}

fn app() -> crate::app::App {
    crate::app::App::new().0
}

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

#[test]
fn selected_worktree_trigger_label_uses_matching_option_or_fallback() {
    let mut app = app();
    app.selected_git_worktree_directory = Some("/repo/beta".to_string());
    app.git_worktree_options = vec![
        worktree("/repo/alpha", "alpha", Some("main")),
        worktree("/repo/beta", "beta worktree · metadata", Some("feature")),
    ];

    assert_eq!(super::header::selected_worktree_trigger_label(&app), "beta w…");

    app.selected_git_worktree_directory = Some("/repo/missing".to_string());
    assert_eq!(super::header::selected_worktree_trigger_label(&app), "工作区");
}

#[test]
fn header_icon_tip_helpers_reflect_summary_and_fullscreen_state() {
    assert_eq!(
        super::header::summary_toggle_icon_and_tip(false),
        (crate::app::assets::Icon::Eye, "显示摘要输入框".to_string())
    );
    assert_eq!(
        super::header::summary_toggle_icon_and_tip(true),
        (crate::app::assets::Icon::EyeSlash, "隐藏摘要输入框".to_string())
    );
    assert_eq!(
        super::header::fullscreen_icon_and_tip(false),
        (crate::app::assets::Icon::Fullscreen, "全屏".to_string())
    );
    assert_eq!(
        super::header::fullscreen_icon_and_tip(true),
        (crate::app::assets::Icon::FullscreenExit, "退出全屏".to_string())
    );
}

#[test]
fn header_toggle_message_helpers_flip_current_state() {
    match super::header::worktree_menu_toggle_message(false) {
        Message::Git(message::GitMessage::ToggleGitWorktreeMenu(value)) => assert!(value),
        _ => panic!("expected worktree toggle message"),
    }
    match super::header::worktree_menu_toggle_message(true) {
        Message::Git(message::GitMessage::ToggleGitWorktreeMenu(value)) => assert!(!value),
        _ => panic!("expected worktree toggle message"),
    }
    match super::header::filter_options_toggle_message(false) {
        Message::Git(message::GitMessage::ToggleFilterOptions(value)) => assert!(value),
        _ => panic!("expected filter toggle message"),
    }
    match super::header::filter_options_toggle_message(true) {
        Message::Git(message::GitMessage::ToggleFilterOptions(value)) => assert!(!value),
        _ => panic!("expected filter toggle message"),
    }
}

#[test]
fn view_builds_with_default_branch_and_no_worktree_picker() {
    let app = app();

    let _element = super::header::view(&app, vec!["src/lib.rs".to_string()]);
}

#[test]
fn view_builds_with_long_branch_fullscreen_and_open_worktree_menu() {
    let mut app = app();
    app.selected_branch = Some("feature/super-long-branch-name".to_string());
    app.git_diff_fullscreen = true;
    app.show_git_diff_summary = true;
    app.show_git_filter_options = true;
    app.git_worktree_menu_open = true;
    app.selected_git_worktree_directory = Some("/repo/wt-two".to_string());
    app.git_worktree_options = vec![
        worktree("/repo/main", "main worktree · extra metadata", Some("main")),
        worktree(
            "/repo/wt-two",
            "secondary worktree with very long label · detail",
            Some("feature/extremely-long-branch"),
        ),
    ];

    let _element =
        super::header::view(&app, vec!["src/lib.rs".to_string(), "src/main.rs".to_string()]);
}

#[test]
fn view_builds_worktree_picker_without_matching_selected_directory_or_branch() {
    let mut app = app();
    app.selected_git_worktree_directory = Some("/repo/missing".to_string());
    app.git_worktree_options = vec![
        worktree("/repo/a", "alpha", None),
        worktree("/repo/b", "beta · ignored suffix", Some("")),
    ];

    let _element = super::header::view(&app, Vec::new());
}
