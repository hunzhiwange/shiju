//! 验证项目常规编辑页的图标颜色、预览标签和交互状态。
//!
//! 预览夹具按图标、名称和颜色优先级构造，非法颜色必须使用主题回退。

use super::*;
use crate::app::message::project::ProjectMessage;
use iced::Color;

fn assert_color_near(actual: Color, expected: Color) {
    let epsilon = 0.001;
    assert!((actual.r - expected.r).abs() < epsilon, "red mismatch: {actual:?}");
    assert!((actual.g - expected.g).abs() < epsilon, "green mismatch: {actual:?}");
    assert!((actual.b - expected.b).abs() < epsilon, "blue mismatch: {actual:?}");
    assert!((actual.a - expected.a).abs() < epsilon, "alpha mismatch: {actual:?}");
}

fn test_app() -> App {
    let (app, _task) = App::new();
    app
}

#[test]
fn icon_color_preset_selection_ignores_whitespace_hash_and_ascii_case() {
    assert!(icon_color_preset_is_selected("  #60A5FA  ", "#60a5fa"));
    assert!(icon_color_preset_is_selected("60a5fa", "#60A5FA"));
    assert!(!icon_color_preset_is_selected("#34d399", "#60a5fa"));
}

#[test]
fn current_icon_color_accepts_rgba_and_hashless_hex_inputs() {
    assert_color_near(
        current_icon_color("#60a5fa80"),
        Color::from_rgba8(0x60, 0xa5, 0xfa, 128.0 / 255.0),
    );
    assert_color_near(current_icon_color("34d399"), Color::from_rgb8(0x34, 0xd3, 0x99));
}

#[test]
fn current_icon_color_falls_back_for_invalid_input() {
    assert_color_near(current_icon_color("bad-color"), DEFAULT_ICON_COLOR);
}

#[test]
fn preview_badge_color_accepts_hex_and_falls_back_for_non_hex_input() {
    assert_color_near(preview_badge_color(" #EF4444 "), Color::from_rgb8(0xef, 0x44, 0x44));
    assert_color_near(preview_badge_color("#60a5fa80"), DEFAULT_ICON_COLOR);
    assert_color_near(preview_badge_color("bad-color"), DEFAULT_ICON_COLOR);
}

#[test]
fn preview_label_prefers_trimmed_icon_then_project_name() {
    assert_eq!(preview_label("  图标  ", "Project"), "图");
    assert_eq!(preview_label("   ", "项目"), "项");
}

#[test]
fn preview_label_defaults_when_icon_and_name_are_empty() {
    assert_eq!(preview_label("", ""), "P");
}

#[test]
fn icon_preview_press_clears_only_a_hovered_image() {
    assert!(matches!(
        icon_preview_press_message(true, true),
        Message::Project(ProjectMessage::ProjectEditIconChanged(value)) if value.is_empty()
    ));
    assert!(matches!(
        icon_preview_press_message(true, false),
        Message::Project(ProjectMessage::ProjectEditIconPickFile)
    ));
    assert!(matches!(
        icon_preview_press_message(false, true),
        Message::Project(ProjectMessage::ProjectEditIconPickFile)
    ));
}

#[test]
fn general_tab_builds_with_default_text_preview() {
    let mut app = test_app();
    app.project_edit_name = "Project".to_string();
    app.project_edit_icon = String::new();
    app.project_edit_icon_color = "#60a5fa".to_string();

    let element = general_tab(&app, "/fixture/project");

    std::hint::black_box(element);
}

#[test]
fn general_tab_builds_with_icon_text_preview_and_color_fallbacks() {
    let mut app = test_app();
    app.project_edit_name = String::new();
    app.project_edit_icon = "  Zed  ".to_string();
    app.project_edit_icon_color = "bad-color".to_string();

    let element = general_tab(&app, "/fixture/project");

    std::hint::black_box(element);
}

#[test]
fn general_tab_builds_image_preview_and_hover_clear_overlay() {
    let path = std::env::temp_dir()
        .join(format!("vibe-window-project-edit-general-{}.png", std::process::id()));
    std::fs::write(&path, b"not a real png but enough for a path handle").unwrap();

    let mut app = test_app();
    app.project_edit_name = "Image".to_string();
    app.project_edit_icon = path.to_string_lossy().into_owned();
    app.project_edit_icon_hovered = true;
    app.project_edit_icon_color = "rgb(96, 165, 250)".to_string();

    let element = general_tab(&app, "/fixture/project");

    std::hint::black_box(element);
    let _ = std::fs::remove_file(path);
}

#[test]
fn general_tab_builds_unhovered_image_preview_without_clear_overlay() {
    let path = std::env::temp_dir()
        .join(format!("vibe-window-project-edit-general-unhovered-{}.png", std::process::id()));
    std::fs::write(&path, b"image path fixture").unwrap();

    let mut app = test_app();
    app.project_edit_name = "Image".to_string();
    app.project_edit_icon = path.to_string_lossy().into_owned();
    app.project_edit_icon_hovered = false;

    let element = general_tab(&app, "/fixture/project");

    std::hint::black_box(element);
    let _ = std::fs::remove_file(path);
}

#[tokio::test]
async fn project_edit_general_form_controls_render_and_update_draft() {
    use crate::app::components::system_settings_common::tab_appearance_tests;
    use crate::app::i18n::I18n;
    use crate::app::state::ProjectEditTab;
    use iced::widget::Space;

    fn overlay(app: &App) -> Element<'_, Message> {
        super::super::with_project_edit(
            app,
            Space::new().width(Length::Fill).height(Length::Fill).into(),
        )
    }

    tab_appearance_tests::run_at_least(
        concat!(module_path!(), "::project_edit_general_form_controls_render_and_update_draft"),
        420,
        test_app,
        |scene, app| {
            app.project_edit_path = Some(format!("/fixture/{}", "nested-project/".repeat(30)));
            app.project_edit_tab = ProjectEditTab::General;
            app.project_edit_name = "测试项目 · Sample project".into();
            app.project_edit_icon = "圖".into();
            app.project_edit_icon_color = "#60a5fa".into();

            let name = "修訂项目 · Renamed";
            let message = scene.edit_modal_input(overlay(app), &app.project_edit_name, name);
            assert!(matches!(&message, Message::Project(ProjectMessage::ProjectEditNameChanged(value)) if value == name));
            let _ = app.update(message);
            assert_eq!(app.project_edit_name, name);

            let icon = "檔";
            let message = scene.edit_modal_input(overlay(app), &app.project_edit_icon, icon);
            assert!(matches!(&message, Message::Project(ProjectMessage::ProjectEditIconChanged(value)) if value == icon));
            let _ = app.update(message);
            assert_eq!(app.project_edit_icon, icon);

            let i18n = I18n::new(app.desktop_locale);
            scene.check_modal_scroll(overlay(app), i18n.__("编辑项目"), i18n.__("保存"));
            assert!(matches!(
                scene.click_message(overlay(app), i18n.__("保存")),
                Message::Project(ProjectMessage::ProjectEditSaved)
            ));

            app.project_edit_tab = ProjectEditTab::Refresh;
            app.project_edit_session_refresh_interval_seconds_input = "23".into();
            let message = scene.edit_modal_input(overlay(app), "23", "57");
            assert!(matches!(&message, Message::Project(ProjectMessage::ProjectEditSessionRefreshIntervalSecondsInputChanged(value)) if value == "57"));
            let _ = app.update(message);
            assert_eq!(app.project_edit_session_refresh_interval_seconds_input, "57");

            let message = scene.click_message(overlay(app), i18n.__("取消"));
            assert!(matches!(message, Message::Project(ProjectMessage::ProjectEditCanceled)));
            let _ = app.update(message);
            assert!(app.project_edit_path.is_none());
        },
    )
    .await;
}
