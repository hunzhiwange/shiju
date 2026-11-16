// 保留宿主契约：共用设置表面接入 App 编辑消息、指标提交与模态关闭持久化；真实字体加载与绘制夹具归desktop-editor。
use crate::app::message::editor::{self, EditorMessage};
use crate::app::{App, Message};
use config_ui::GatewayClientSystemSettingsConfig;
use iced::widget::{container, text};
use iced::{Element, Length, Point, Size, Theme};
#[path = "../../../../libs/desktop-editor/src/settings_test_support.rs"]
mod rendering;
use rendering::{bounds, click, position, renderer, scene};

fn isolated(test: &str) -> bool {
    const CHILD: &str = "SHIJU_ECP03_CHILD";
    if std::env::var_os(CHILD).is_some() {
        return true;
    }
    let home = tempfile::tempdir().unwrap();
    let config = GatewayClientSystemSettingsConfig {
        host: "[".into(),
        servers: Vec::new(),
        ..Default::default()
    };
    std::fs::write(
        home.path().join("gateway-client-bootstrap.json"),
        serde_json::to_vec(&config).unwrap(),
    )
    .unwrap();
    let test = format!("{}::{test}", module_path!().split_once("::").unwrap().1);
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", &test])
        .env(CHILD, "1")
        .env("VIBEWINDOW_CONFIG_DIR", home.path())
        .env("VIBEWINDOW_WORKSPACE", home.path())
        .current_dir(home.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("1 passed; 0 failed"),
        "{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    false
}

fn fixture() -> App {
    assert_eq!(crate::app::config::gateway_client_endpoint().normalized_host(), "[");
    App::new().0
}

fn preview(app: &App) -> Element<'_, Message> {
    let base = container(
        iced::widget::mouse_area(
            container(text("Preview fixture")).width(Length::Fill).height(Length::Fill),
        )
        .on_press(Message::Editor(EditorMessage::ClosePopover)),
    )
    .width(Length::Fill)
    .height(Length::Fill);
    crate::app::components::preview_panel::build_settings_overlay(app, base.into())
}

#[tokio::test]
async fn ecp03_settings_surfaces_apply_identical_metrics_and_theme() {
    if !isolated("ecp03_settings_surfaces_apply_identical_metrics_and_theme") {
        return;
    }
    let mut app = fixture();
    app.current_font_size = 16.0;
    app.current_line_height = 26.0;
    app.auto_adjust_line_height = false;
    app.editor_follow_system_theme = true;
    app.app_theme = Theme::Dark;
    app.editor_theme = Theme::Light;
    let mut renderer = renderer().await;
    for local in [true, false] {
        let element =
            if local { preview(&app) } else { super::system_settings_editor::content(&app) };
        let mut ui = scene(element, &mut renderer, Size::new(640, 1200));
        let controls = bounds(&mut ui, &renderer);
        assert_eq!(controls.focus.len(), 1, "manual theme disabled; language remains available");
        let badge = position(&controls.labels, "26.0px");
        let end = Point::new(badge.x - 35.0, badge.center_y());
        let messages = click(&mut ui, &mut renderer, end);
        assert!(
            messages
                .iter()
                .any(|m| matches!(m, Message::Editor(EditorMessage::LineHeightPreview(_))))
        );
        drop(ui);
        for message in messages {
            if let Message::Editor(message) = message {
                let _task = editor::update(&mut app, message);
            }
        }
        assert!(app.current_line_height > 50.0);
        let _task = editor::update(&mut app, EditorMessage::ToggleFollowSystemTheme(false));
        let _task = editor::update(&mut app, EditorMessage::ThemeChanged(Theme::Dark));
        assert_eq!(app.effective_editor_theme(), Theme::Dark);
        app.current_line_height = 26.0;
        app.editor_follow_system_theme = true;
    }
}
#[tokio::test]
async fn ecp03_closing_modal_commits_pending_metrics() {
    if !isolated("ecp03_closing_modal_commits_pending_metrics") {
        return;
    }
    let mut app = fixture();
    let _task = editor::update(&mut app, EditorMessage::ToggleSettings);
    let _task = editor::update(&mut app, EditorMessage::FontSizePreview(22.0));
    let _task = editor::update(&mut app, EditorMessage::LineHeightPreview(60.0));
    let _task = editor::update(&mut app, EditorMessage::ToggleSettings);
    assert!(!app.show_preview_settings);
    assert_eq!(app.current_font_size, 22.0);
    assert_eq!(app.current_line_height, 60.0);
    assert_eq!(app.git.git_copy_modal_code_editor.font_size(), 22.0);
    assert_eq!(app.git.git_copy_modal_code_editor.line_height(), 60.0);
}
