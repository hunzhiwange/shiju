#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("new_session_tests"));
}

use super::*;
use crate::app::message::ProjectMessage;

fn test_app() -> crate::app::App {
    crate::app::App::new().0
}

fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

fn color_from_background(background: Option<Background>) -> Color {
    match background {
        Some(Background::Color(color)) => color,
        _ => panic!("expected color background"),
    }
}

fn assert_color_near(actual: Color, expected: Color) {
    let epsilon = 0.001;
    assert!((actual.r - expected.r).abs() < epsilon, "red mismatch: {actual:?}");
    assert!((actual.g - expected.g).abs() < epsilon, "green mismatch: {actual:?}");
    assert!((actual.b - expected.b).abs() < epsilon, "blue mismatch: {actual:?}");
    assert!((actual.a - expected.a).abs() < epsilon, "alpha mismatch: {actual:?}");
}

#[test]
fn is_dark_theme_detects_palette_brightness() {
    assert!(is_dark_theme(&Theme::Dark));
    assert!(!is_dark_theme(&Theme::Light));
}

#[test]
fn worktree_action_button_background_uses_theme_base_and_danger_states() {
    assert_color_near(
        worktree_action_button_background(&Theme::Dark, iced::widget::button::Status::Active),
        Color::from_rgba(0.34, 0.36, 0.40, 0.92),
    );
    assert_color_near(
        worktree_action_button_background(&Theme::Light, iced::widget::button::Status::Disabled),
        Color::from_rgba(0.72, 0.74, 0.78, 0.95),
    );
    assert_color_near(
        worktree_action_button_background(&Theme::Light, iced::widget::button::Status::Hovered),
        Color::from_rgb8(220, 38, 38),
    );
    assert_color_near(
        worktree_action_button_background(&Theme::Dark, iced::widget::button::Status::Pressed),
        Color::from_rgb8(185, 28, 28),
    );
}

#[test]
fn worktree_action_button_style_keeps_white_text_and_round_border() {
    let style = worktree_action_button_style(&Theme::Dark, iced::widget::button::Status::Active);

    assert_eq!(style.text_color, Color::WHITE);
    assert_eq!(style.border.width, 0.0);
    assert_eq!(style.border.color, Color::TRANSPARENT);
    assert_color_near(
        color_from_background(style.background),
        Color::from_rgba(0.34, 0.36, 0.40, 0.92),
    );
}

#[test]
fn create_worktree_button_style_uses_blue_state_colors() {
    let active = create_worktree_button_style(&Theme::Light, iced::widget::button::Status::Active);
    let hovered =
        create_worktree_button_style(&Theme::Light, iced::widget::button::Status::Hovered);
    let pressed =
        create_worktree_button_style(&Theme::Light, iced::widget::button::Status::Pressed);
    let disabled =
        create_worktree_button_style(&Theme::Light, iced::widget::button::Status::Disabled);

    assert_color_near(color_from_background(active.background), Color::from_rgb8(59, 130, 246));
    assert_color_near(color_from_background(hovered.background), Color::from_rgb8(37, 99, 235));
    assert_color_near(color_from_background(pressed.background), Color::from_rgb8(29, 78, 216));
    assert_color_near(color_from_background(disabled.background), Color::from_rgb8(59, 130, 246));
}

#[test]
fn list_item_button_style_is_transparent_until_hover_or_press() {
    let active = list_item_button_style(&Theme::Light, iced::widget::button::Status::Active);
    let hovered = list_item_button_style(&Theme::Light, iced::widget::button::Status::Hovered);
    let pressed = list_item_button_style(&Theme::Light, iced::widget::button::Status::Pressed);

    assert_eq!(color_from_background(active.background), Color::TRANSPARENT);
    assert_ne!(color_from_background(hovered.background), Color::TRANSPARENT);
    assert_ne!(color_from_background(pressed.background), Color::TRANSPARENT);
}

#[test]
fn neutral_and_close_button_styles_use_theme_text_and_background() {
    let neutral = neutral_button_style(&Theme::Dark, iced::widget::button::Status::Active);
    let close = close_button_style(&Theme::Dark, iced::widget::button::Status::Hovered);

    assert_eq!(neutral.text_color, Theme::Dark.palette().text);
    assert_eq!(close.text_color, Theme::Dark.palette().text);
    assert_ne!(color_from_background(neutral.background), Color::TRANSPARENT);
    assert_ne!(color_from_background(close.background), Color::TRANSPARENT);
}

#[test]
fn danger_soft_button_style_scales_red_background_by_status() {
    let active = danger_soft_button_style(&Theme::Light, iced::widget::button::Status::Active);
    let hovered = danger_soft_button_style(&Theme::Light, iced::widget::button::Status::Hovered);
    let pressed = danger_soft_button_style(&Theme::Light, iced::widget::button::Status::Pressed);

    assert_eq!(active.text_color, Color::from_rgb8(220, 38, 38));
    assert_color_near(
        color_from_background(active.background),
        Color::from_rgb8(220, 38, 38).scale_alpha(0.12),
    );
    assert_color_near(
        color_from_background(hovered.background),
        Color::from_rgb8(220, 38, 38).scale_alpha(0.18),
    );
    assert_color_near(
        color_from_background(pressed.background),
        Color::from_rgb8(220, 38, 38).scale_alpha(0.26),
    );
}

#[test]
fn panel_styles_define_visible_backgrounds_and_borders() {
    let confirmation = confirmation_panel_style(&Theme::Light);
    let picker = picker_panel_style(&Theme::Dark);
    let overlay = overlay_style(&Theme::Light);

    assert!(confirmation.background.is_some());
    assert_eq!(confirmation.border.width, 1.0);
    assert!(picker.background.is_some());
    assert_eq!(picker.border.width, 1.0);
    assert_color_near(
        color_from_background(overlay.background),
        Color::from_rgba(0.04, 0.05, 0.07, 0.28),
    );
}

#[test]
fn worktree_display_name_uses_file_name_when_available() {
    assert_eq!(worktree_display_name("/repo/worktrees/feature-login"), "feature-login");
    assert_eq!(worktree_display_name("plain-name"), "plain-name");
}

#[test]
fn worktree_display_name_preserves_root_when_file_name_is_unavailable() {
    assert_eq!(worktree_display_name("/"), "/");
}

#[test]
fn project_display_title_prefers_non_empty_edit_name() {
    let projects = vec!["/repo/a".to_owned(), "/repo/b".to_owned()];
    let edits = vec![" Alpha ".to_owned(), "   ".to_owned()];

    assert_eq!(project_display_title("/repo/a", &projects, &edits), " Alpha ");
    assert_eq!(project_display_title("/repo/b", &projects, &edits), "/repo/b");
    assert_eq!(project_display_title("/repo/a", &projects, &[]), "/repo/a");
    assert_eq!(project_display_title("/repo/missing", &projects, &edits), "/repo/missing");
}

#[test]
fn create_session_message_targets_project_path() {
    match create_session_message("/repo/app") {
        Message::Project(ProjectMessage::ProjectCreateSession(path)) => {
            assert_eq!(path, "/repo/app")
        }
        other => panic!("unexpected message: {other:?}"),
    }
}

#[test]
fn pick_session_message_carries_project_and_directory() {
    match pick_session_message("/repo/app", "/repo/app-wt") {
        Message::Project(ProjectMessage::ProjectCreateSessionPicked {
            project_path,
            directory,
        }) => {
            assert_eq!(project_path, "/repo/app");
            assert_eq!(directory, "/repo/app-wt");
        }
        other => panic!("unexpected message: {other:?}"),
    }
}

#[test]
fn worktree_messages_carry_expected_payloads() {
    match create_worktree_message("/repo/app") {
        Message::Project(ProjectMessage::ProjectCreateSessionWorktree(path)) => {
            assert_eq!(path, "/repo/app");
        }
        other => panic!("unexpected message: {other:?}"),
    }
    match worktree_name_changed_message("feature".to_owned()) {
        Message::Project(ProjectMessage::ProjectCreateSessionWorktreeNameChanged(name)) => {
            assert_eq!(name, "feature");
        }
        other => panic!("unexpected message: {other:?}"),
    }
    match delete_worktree_message("/repo/app-wt") {
        Message::Project(ProjectMessage::ProjectCreateSessionDeleteWorktree(directory)) => {
            assert_eq!(directory, "/repo/app-wt");
        }
        other => panic!("unexpected message: {other:?}"),
    }
    match reset_worktree_message("/repo/app-wt") {
        Message::Project(ProjectMessage::ProjectCreateSessionResetWorktree(directory)) => {
            assert_eq!(directory, "/repo/app-wt");
        }
        other => panic!("unexpected message: {other:?}"),
    }
}

#[test]
fn new_session_button_builds_inactive_and_active_states() {
    let mut app = test_app();

    keep_element(new_session_button(&app, "/repo/app".to_owned()));

    app.new_session_picker_project = Some("/repo/app".to_owned());
    keep_element(new_session_button(&app, "/repo/app".to_owned()));
}

#[test]
fn picker_layer_builds_empty_overlay_when_no_project_selected() {
    let app = test_app();

    keep_element(new_session_picker_layer(&app));
}

#[test]
fn picker_layer_builds_loading_state_for_selected_project() {
    let mut app = test_app();
    app.new_session_picker_project = Some("/repo/app".to_owned());
    app.recent_projects = vec!["/repo/app".to_owned()];
    app.recent_projects_edits = vec!["App".to_owned()];

    keep_element(new_session_picker_layer(&app));
}

#[test]
fn picker_layer_builds_worktree_options_and_create_form() {
    let mut app = test_app();
    app.new_session_picker_project = Some("/repo/app".to_owned());
    app.new_session_picker_options = vec![
        ("/repo/app".to_owned(), "主工作区".to_owned()),
        ("/repo/app-feature".to_owned(), "feature".to_owned()),
        ("__create_worktree__".to_owned(), "创建新工作区".to_owned()),
    ];
    app.new_session_worktree_name = "feature-login".to_owned();

    keep_element(new_session_picker_layer(&app));
}

#[test]
fn picker_layer_builds_error_and_confirmation_panels() {
    let mut app = test_app();
    app.new_session_picker_project = Some("/repo/app".to_owned());
    app.new_session_picker_options = vec![("/repo/app-feature".to_owned(), "feature".to_owned())];
    app.new_session_delete_error = Some("dirty worktree".to_owned());
    app.new_session_reset_error = Some("reset failed".to_owned());
    app.new_session_confirm_delete_directory = Some("/repo/app-feature".to_owned());
    app.new_session_force_delete_directory = Some("/repo/app-force".to_owned());
    app.new_session_confirm_reset_directory = Some("/repo/app-reset".to_owned());

    keep_element(new_session_picker_layer(&app));
}

#[test]
fn picker_layer_builds_with_long_custom_project_title_and_small_window_height() {
    let mut app = test_app();
    app.window_size = (900.0, 120.0);
    app.new_session_picker_project = Some("/repo/app".to_owned());
    app.recent_projects = vec!["/repo/app".to_owned()];
    app.recent_projects_edits = vec!["非常非常非常非常非常非常非常非常长的项目显示名称".to_owned()];
    app.new_session_picker_options = vec![("/repo/app".to_owned(), "主工作区".to_owned())];

    keep_element(new_session_picker_layer(&app));
}
