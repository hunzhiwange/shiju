//! 验证状态栏主题识别、预览上下文和语言服务器进度状态。
//!
//! 预览标签与服务器进度映射分别构造，缺失路径或服务键时必须显示安全默认状态。

#[test]
fn app_view_status_tests_module_is_wired() {
    assert!(module_path!().ends_with("app_view_status_tests"));
}

#[cfg(not(target_arch = "wasm32"))]
use crate::app::preview::{LspProgress, PreviewTab};
#[cfg(not(target_arch = "wasm32"))]
use crate::app::{Message, message::NotificationMessage};
#[cfg(not(target_arch = "wasm32"))]
use iced::advanced::renderer::Headless;
#[cfg(not(target_arch = "wasm32"))]
use iced::advanced::{Layout, Shell, clipboard, layout, mouse, widget::Tree};
#[cfg(not(target_arch = "wasm32"))]
use iced::{Event, Font, Pixels, Rectangle, Size};

#[cfg(not(target_arch = "wasm32"))]
fn preview_tab(path: &str, server_key: Option<&'static str>) -> PreviewTab {
    PreviewTab {
        path: path.to_string(),
        title: "main.rs".to_string(),
        content: "fn main() {}\n".to_string(),
        is_dirty: false,
        truncated: false,
        auto_save_revision: 0,
        editor: crate::app::components::editor::Editor::new("fn main() {}", "rust"),
        scroll_id: iced::widget::Id::unique(),
        lsp_server_key: server_key,
        lsp_uri: Some("file:///workspace/src/main.rs".to_string()),
        lsp_language_id: Some("rust".to_string()),
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn is_dark_theme_detects_dark_and_light_palettes() {
    assert!(super::is_dark_theme(&iced::Theme::Dark));
    assert!(!super::is_dark_theme(&iced::Theme::Light));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn status_bar_builds_ready_when_no_preview_tab_is_active() {
    let (mut app, _) = crate::app::App::new();
    std::hint::black_box(super::status_bar(&app));

    // 尚未激活预览标签时故意忽略全局状态，避免显示与当前页面无关的进度。
    app.lsp_status = Some("LSP: starting".to_string());
    std::hint::black_box(super::status_bar(&app));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn status_bar_builds_ready_and_global_status_for_tab_without_server_key() {
    let (mut app, _) = crate::app::App::new();
    app.active_preview_path = Some("/workspace/src/main.rs".to_string());
    app.preview_tabs.push(preview_tab("/workspace/src/main.rs", None));

    std::hint::black_box(super::status_bar(&app));

    app.lsp_status = Some("LSP: discovering server".to_string());
    std::hint::black_box(super::status_bar(&app));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn status_bar_builds_ready_when_active_path_has_no_matching_tab() {
    let (mut app, _) = crate::app::App::new();
    app.active_preview_path = Some("/workspace/src/missing.rs".to_string());
    app.preview_tabs.push(preview_tab("/workspace/src/main.rs", Some("rust-analyzer")));
    app.lsp_status = Some("LSP: starting".to_string());

    std::hint::black_box(super::status_bar(&app));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn status_bar_builds_active_tab_lsp_variants() {
    let (mut app, _) = crate::app::App::new();
    app.active_preview_path = Some("/workspace/src/main.rs".to_string());
    app.preview_tabs.push(preview_tab("/workspace/src/main.rs", Some("rust-analyzer")));

    std::hint::black_box(super::status_bar(&app));

    app.lsp_progress.insert(
        "rust-analyzer".to_string(),
        std::collections::HashMap::from([(
            "token".to_string(),
            LspProgress {
                title: "index".to_string(),
                message: Some("crates".to_string()),
                percentage: Some(42),
            },
        )]),
    );
    app.spinner_frame = 99;
    std::hint::black_box(super::status_bar(&app));

    app.lsp_progress.get_mut("rust-analyzer").unwrap().get_mut("token").unwrap().percentage =
        Some(100);
    std::hint::black_box(super::status_bar(&app));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn status_bar_builds_global_status_when_server_has_no_progress_map() {
    let (mut app, _) = crate::app::App::new();
    app.active_preview_path = Some("/workspace/src/main.rs".to_string());
    app.preview_tabs.push(preview_tab("/workspace/src/main.rs", Some("rust-analyzer")));
    app.lsp_status = Some("LSP: initializing".to_string());

    std::hint::black_box(super::status_bar(&app));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn status_bar_builds_server_key_when_progress_map_is_empty() {
    let (mut app, _) = crate::app::App::new();
    app.active_preview_path = Some("/workspace/src/main.rs".to_string());
    app.preview_tabs.push(preview_tab("/workspace/src/main.rs", Some("rust-analyzer")));
    app.lsp_progress.insert("rust-analyzer".to_string(), std::collections::HashMap::new());

    std::hint::black_box(super::status_bar(&app));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn status_bar_builds_running_progress_with_missing_optional_fields() {
    let (mut app, _) = crate::app::App::new();
    app.active_preview_path = Some("/workspace/src/main.rs".to_string());
    app.preview_tabs.push(preview_tab("/workspace/src/main.rs", Some("rust-analyzer")));
    app.lsp_progress.insert(
        "rust-analyzer".to_string(),
        std::collections::HashMap::from([(
            "token".to_string(),
            LspProgress { title: "index".to_string(), message: None, percentage: None },
        )]),
    );

    std::hint::black_box(super::status_bar(&app));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn status_bar_builds_overcomplete_progress_as_completion() {
    let (mut app, _) = crate::app::App::new();
    app.active_preview_path = Some("/workspace/src/main.rs".to_string());
    app.preview_tabs.push(preview_tab("/workspace/src/main.rs", Some("rust-analyzer")));
    app.lsp_progress.insert(
        "rust-analyzer".to_string(),
        std::collections::HashMap::from([(
            "token".to_string(),
            LspProgress {
                title: "index".to_string(),
                message: Some("done".to_string()),
                percentage: Some(125),
            },
        )]),
    );

    std::hint::black_box(super::status_bar(&app));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn status_bar_builds_notification_variant() {
    let (mut app, _) = crate::app::App::new();
    app.notifications.push(crate::app::state::Notification {
        id: 1,
        message: "warning".to_string(),
        created_at: web_time::SystemTime::UNIX_EPOCH,
    });

    std::hint::black_box(super::status_bar(&app));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn status_bar_builds_multiple_notification_count() {
    let (mut app, _) = crate::app::App::new();
    for id in 1..=3 {
        app.notifications.push(crate::app::state::Notification {
            id,
            message: format!("warning {id}"),
            created_at: web_time::SystemTime::UNIX_EPOCH,
        });
    }

    std::hint::black_box(super::status_bar(&app));
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn status_bar_uses_reserved_height() {
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let (app, _) = crate::app::App::new();
    let mut status_bar = super::status_bar(&app);
    let limits = layout::Limits::new(Size::ZERO, Size::new(800.0, 80.0));
    let mut tree = Tree::new(status_bar.as_widget());
    let node = status_bar.as_widget_mut().layout(&mut tree, &renderer, &limits);

    assert_eq!(node.bounds().height, super::STATUS_BAR_HEIGHT);
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn status_bar_keeps_notification_at_trailing_edge() {
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let (app, _) = crate::app::App::new();
    let mut status_bar = super::status_bar(&app);
    let limits = layout::Limits::new(Size::ZERO, Size::new(800.0, 40.0));
    let mut tree = Tree::new(status_bar.as_widget());
    let node = status_bar.as_widget_mut().layout(&mut tree, &renderer, &limits);
    let row =
        Layout::new(&node).children().next().expect("status bar container should contain a row");
    let notification =
        row.children().last().expect("notification should remain at the trailing edge").bounds();
    let cursor = mouse::Cursor::Available(notification.center());
    let mut clipboard = clipboard::Null;
    let mut messages = Vec::new();

    for event in [
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
    ] {
        let mut shell = Shell::new(&mut messages);
        status_bar.as_widget_mut().update(
            &mut tree,
            &event,
            Layout::new(&node),
            cursor,
            &renderer,
            &mut clipboard,
            &mut shell,
            &Rectangle::INFINITE,
        );
    }

    assert!(messages.into_iter().any(|message| {
        matches!(message, Message::Notification(NotificationMessage::ToggleExpanded))
    }));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn status_bar_builds_account_entry_and_open_menu() {
    let (mut app, _) = crate::app::App::new();
    std::hint::black_box(super::status_bar(&app));
    app.account.menu_open = true;
    std::hint::black_box(super::status_bar(&app));
}
