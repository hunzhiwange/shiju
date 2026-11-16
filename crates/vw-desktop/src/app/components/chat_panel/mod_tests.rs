//! 验证聊天面板的阅读列布局、消息虚拟化与典型视觉场景构造。
//! 多视口宽度和多主题场景用于守住共享阅读宽度、渲染窗口及单一状态所有权。

use super::{rough_message_heights, view};
use crate::app::components::chat_panel::virtualizer::ChatVirtualizer;
use crate::app::models::{ChatMessage, ChatRole};
use crate::app::{App, Message};
#[cfg(not(target_arch = "wasm32"))]
use iced::advanced::renderer::Headless;
#[cfg(not(target_arch = "wasm32"))]
use iced::advanced::{Layout, layout, widget::Tree};
#[cfg(not(target_arch = "wasm32"))]
use iced::{Font, Pixels, Size};

fn message(role: ChatRole, content: &str) -> ChatMessage {
    ChatMessage {
        role,
        content: content.to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }
}

#[test]
fn conversation_shell_borrows_runtime_once() {
    let mut app = App::new().0;
    app.active_session_id = Some("session".to_string());
    app.current_session_runtime_mut().request_lifecycle.is_requesting = true;
    app.chat = vec![message(ChatRole::User, "one"), message(ChatRole::Assistant, "two")];
    super::performance::reset();

    let _ = view(&app);

    assert_eq!(super::performance::snapshot().runtime_borrows, 1);
    assert_eq!(super::performance::snapshot().runtime_clones, 0);
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn conversation_shell_renders_visual_regression_scenarios_without_panic() {
    // 场景覆盖内容规模、运行状态和权限失败；主题与皮肤在外层形成组合矩阵。
    #[derive(Clone, Copy)]
    enum Scenario {
        Empty,
        Short,
        Long,
        Running,
        WaitingPermission,
        ToolFailure,
        Narrow,
    }

    impl Scenario {
        fn name(self) -> &'static str {
            match self {
                Self::Empty => "empty",
                Self::Short => "short",
                Self::Long => "long",
                Self::Running => "running",
                Self::WaitingPermission => "waiting-permission",
                Self::ToolFailure => "tool-failure",
                Self::Narrow => "narrow",
            }
        }
    }

    fn visual_app(scenario: Scenario, theme: iced::Theme, skinned: bool) -> App {
        let (mut app, _) = App::new();
        app.app_theme = theme;

        if skinned {
            let background = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../libs/desktop-theme/assets/presets/tahoe-beach-night/background.webp");
            assert!(background.is_file(), "visual regression skin asset should exist");
            app.active_theme_skin = Some(crate::app::theme_catalog::ActiveThemeSkin {
                theme_id: "batch-6-visual-regression".to_string(),
                background_path: Some(background.clone()),
                cover_path: None,
                chat_header_path: None,
                chat_body_path: Some(background),
                chat_input_path: None,
                chrome: crate::app::theme_catalog::ThemeChrome::default(),
            });
        }

        if matches!(scenario, Scenario::Empty) {
            return app;
        }

        app.active_session_id = Some(format!("batch-6-{}", scenario.name()));
        app.chat = match scenario {
            Scenario::Long => (0..80)
                .map(|idx| {
                    let role = if idx % 2 == 0 { ChatRole::User } else { ChatRole::Assistant };
                    message(
                        role,
                        &format!(
                            "第 {idx} 条消息：用于检查长会话虚拟滚动、中文正文与阅读列宽度。\n\n- item {idx}\n- 第二项"
                        ),
                    )
                })
                .collect(),
            Scenario::Running => vec![
                message(ChatRole::User, "请读取中文路径并总结。"),
                message(
                    ChatRole::Assistant,
                    concat!(
                        "<think>正在分析中文状态与工具摘要</think>\n",
                        "tool read\n",
                        "{\"tool_call_id\":\"call-running\",\"status\":\"running\",",
                        "\"input\":\"{\\\"filePath\\\":\\\"src/中文路径.rs\\\"}\"}"
                    ),
                ),
            ],
            Scenario::WaitingPermission => vec![
                message(ChatRole::User, "请执行需要审批的命令。"),
                message(
                    ChatRole::Assistant,
                    concat!(
                        "tool bash\n",
                        "{\"tool_call_id\":\"call-permission\",\"status\":\"waiting\",",
                        "\"input\":\"{\\\"command\\\":\\\"printf batch-6\\\"}\"}"
                    ),
                ),
            ],
            Scenario::ToolFailure => vec![
                message(ChatRole::User, "运行失败场景。"),
                message(
                    ChatRole::Assistant,
                    concat!(
                        "tool bash\n",
                        "{\"tool_call_id\":\"call-failed\",\"status\":\"failed\",",
                        "\"input\":\"{\\\"command\\\":\\\"echo failed\\\"}\",",
                        "\"error\":\"命令执行失败\"}"
                    ),
                ),
            ],
            Scenario::Short | Scenario::Narrow => vec![
                message(ChatRole::User, "请用简短中文回答这个问题。"),
                message(
                    ChatRole::Assistant,
                    "## 简短回答\n\n正文、`inline code` 与元信息沿同一阅读轴排列。",
                ),
            ],
            Scenario::Empty => unreachable!(),
        };

        if matches!(scenario, Scenario::Running) {
            app.current_session_runtime_mut().request_lifecycle.is_requesting = true;
        }
        if matches!(scenario, Scenario::WaitingPermission) {
            app.chat_message_ids = vec![None, Some("msg-permission".to_string())];
            let request = gateway_interaction::PendingPermissionRequestDto {
                id: "req-permission".to_string(),
                session_id: app.active_session_id.clone().unwrap_or_default(),
                permission: "bash".to_string(),
                patterns: vec![],
                metadata: serde_json::Map::new(),
                always: vec![],
                tool: Some(gateway_interaction::PendingPermissionToolDto {
                    message_id: "msg-permission".to_string(),
                    call_id: "call-permission".to_string(),
                }),
            };
            app.permission_modal_request_id = Some(request.id.clone());
            app.permission_modal_request = Some(request.clone());
            app.permission_modal_requests = vec![request];
        }

        let heights = rough_message_heights(&app.chat);
        app.chat_message_estimated_heights = heights.clone();
        app.chat_height_index = super::height_index::ChatHeightIndex::from_heights(&heights);
        app.chat_virtualizer = ChatVirtualizer::from_estimated_heights(&heights);
        app.chat_virtualizer.set_viewport_height(520.0);
        app.rebuild_active_session_message_meta();
        app
    }

    let scenarios = [
        Scenario::Empty,
        Scenario::Short,
        Scenario::Long,
        Scenario::Running,
        Scenario::WaitingPermission,
        Scenario::ToolFailure,
        Scenario::Narrow,
    ];
    let variants = [
        ("light", iced::Theme::Light, false),
        ("dark", iced::Theme::Dark, false),
        ("skin", iced::Theme::Dark, true),
    ];

    for (variant_name, theme, skinned) in variants {
        for scenario in scenarios {
            let mut renderer =
                <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
                    .await
                    .expect("tiny-skia headless renderer should be available");
            let app = visual_app(scenario, theme.clone(), skinned);
            let width = if matches!(scenario, Scenario::Narrow) { 420.0 } else { 1_000.0 };
            let bounds = Size::new(width, 720.0);
            let shell = iced::widget::column![
                iced::widget::container(view(&app))
                    .width(iced::Length::Fill)
                    .height(iced::Length::Fill),
                iced::widget::container(crate::app::components::input_panel::view(&app))
                    .width(iced::Length::Fill)
                    .padding(iced::Padding { top: 0.0, right: 16.0, bottom: 8.0, left: 16.0 })
            ]
            .width(iced::Length::Fill)
            .height(iced::Length::Fill);
            let mut element: iced::Element<'_, Message> = iced::widget::container(shell)
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .style(|theme: &iced::Theme| iced::widget::container::Style {
                    background: Some(iced::Background::Color(theme.palette().background)),
                    ..Default::default()
                })
                .into();
            let limits = layout::Limits::new(Size::ZERO, bounds);
            let mut tree = Tree::new(element.as_widget());
            let node = element.as_widget_mut().layout(&mut tree, &renderer, &limits);
            assert!(node.bounds().width <= bounds.width);
            assert!(node.bounds().height <= bounds.height);
            let viewport = iced::Rectangle::with_size(bounds);
            element.as_widget().draw(
                &tree,
                &mut renderer,
                &theme,
                &iced::advanced::renderer::Style::default(),
                Layout::new(&node),
                iced::mouse::Cursor::Unavailable,
                &viewport,
            );
            let screenshot_size = Size::new(width as u32, bounds.height as u32);
            let pixels = renderer.screenshot(screenshot_size, 1.0, iced::Color::TRANSPARENT);
            assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));

            if let Ok(directory) = std::env::var("VW_CONVERSATION_VISUAL_SCREENSHOT_DIR") {
                image::save_buffer(
                    std::path::Path::new(&directory)
                        .join(format!("{variant_name}-{}.png", scenario.name())),
                    &pixels,
                    screenshot_size.width,
                    screenshot_size.height,
                    image::ColorType::Rgba8,
                )
                .expect("save conversation visual regression screenshot");
            }
        }
    }
}

#[test]
fn chat_panel_virtualization_has_single_state_owner() {
    let sources = [
        ("chat_panel/mod.rs", include_str!("mod.rs")),
        ("session/chat_ui.rs", include_str!("../../session/chat_ui.rs")),
        ("message/chat/mod.rs", include_str!("../../message/chat/mod.rs")),
        ("message/chat/input.rs", include_str!("../../message/chat/input.rs")),
        ("message/chat/input/shared.rs", include_str!("../../message/chat/input/shared.rs")),
        (
            "message/chat/stream/stream_events.rs",
            include_str!("../../message/chat/stream/stream_events.rs"),
        ),
    ];

    for (path, source) in sources {
        assert!(
            !source.contains("chat_height_index.compute_window("),
            "{path} must get chat windows from ChatVirtualizer"
        );
        assert!(
            !source.contains("set_viewport_height(self.chat_scroll_viewport_h)")
                && !source.contains("set_relative_y(self.chat_scroll_offset_y)"),
            "{path} must not let legacy scroll fields drive ChatVirtualizer"
        );
        assert!(
            !source.contains("app.chat_scroll_offset_y = app.chat_virtualizer.relative_y()")
                && !source.contains(
                    "app.chat_scroll_viewport_h = app.chat_virtualizer.viewport_height_px()"
                ),
            "{path} must mirror ChatVirtualizer through the shared compat sync"
        );
        assert!(
            !source.contains("app.chat_height_index.update_height(")
                && !source.contains("app.chat_message_estimated_heights.get_mut("),
            "{path} must update measured heights through ChatVirtualizer"
        );
    }
}

fn keep_element(element: iced::Element<'_, Message>) {
    std::hint::black_box(element);
}

#[test]
fn chat_panel_view_builds_empty_and_loading_session_states() {
    let (mut app, _) = App::new();
    keep_element(view(&app));

    app.active_session_id = Some("loading".to_string());
    app.active_session_view_state.ui_preparing = true;
    app.active_session_view_state.base_ready = false;
    keep_element(view(&app));
}

#[test]
fn chat_panel_view_builds_started_message_window() {
    let (mut app, _) = App::new();
    app.active_session_id = Some("active".to_string());
    app.chat = vec![message(ChatRole::User, "question"), message(ChatRole::Assistant, "answer")];
    app.chat_virtualizer = ChatVirtualizer::from_estimated_heights(&[80.0, 80.0]);
    app.chat_virtualizer.set_viewport_height(200.0);

    keep_element(view(&app));
}

#[test]
fn chat_panel_switch_preserves_conversation_state() {
    use crate::app::message::chat::{self, ChatMessage as AppChatMessage};
    use crate::app::state::ChatPanelMode;

    let (mut app, _) = App::new();
    app.active_session_id = Some("active".to_string());
    app.chat = vec![message(ChatRole::User, "问题"), message(ChatRole::Assistant, "回答")];
    app.chat_virtualizer = ChatVirtualizer::from_estimated_heights(&[80.0, 120.0]);
    app.chat_virtualizer.set_viewport_height(100.0);
    app.chat_virtualizer.set_scroll_top_px(40.0);
    app.sync_chat_virtualizer_compat_fields();
    let before_window = app.chat_virtualizer.window();
    let before_offset = app.chat_scroll_offset_y;

    let _task =
        chat::update(&mut app, AppChatMessage::ChatPanelModeSelected(ChatPanelMode::Trajectory));
    keep_element(view(&app));
    let _task =
        chat::update(&mut app, AppChatMessage::ChatPanelModeSelected(ChatPanelMode::Conversation));
    keep_element(view(&app));

    assert_eq!(app.chat_virtualizer.window(), before_window);
    assert_eq!(app.chat_scroll_offset_y, before_offset);
    assert_eq!(app.chat.len(), 2);
}

#[test]
fn chat_panel_context_menu_has_a_single_overlay_owner() {
    assert!(
        include_str!("../../../../../libs/desktop-chat-view/src/panel.rs")
            .contains("PointBelowOverlay::new(content, menu)")
    );
    for source in [
        include_str!("message_view.rs"),
        include_str!("tools/advanced_view.rs"),
        include_str!("../../../../../libs/desktop-chat-tools/src/tools/apply_patch_view.rs"),
        include_str!("../../../../../libs/desktop-chat-tools/src/tools/files_view/common_view.rs"),
        include_str!("../../../../../libs/desktop-chat-tools/src/tools/files_view/write_view.rs"),
        include_str!("../../../../../libs/desktop-chat-tools/src/tools/text_view.rs"),
        include_str!("../../../../../libs/desktop-chat-tools/src/tools/todo_views.rs"),
        include_str!("../../../../../libs/desktop-chat-tools/src/tools/web_view.rs"),
    ] {
        assert!(!source.contains("PointBelowOverlay"));
    }
}
