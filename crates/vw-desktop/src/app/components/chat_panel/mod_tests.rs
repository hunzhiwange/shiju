//! 验证聊天面板的阅读列布局、消息虚拟化与典型视觉场景构造。
//! 多视口宽度和多主题场景用于守住共享阅读宽度、渲染窗口及单一状态所有权。

use super::{
    active_user_question_idx, chat_message_column, chat_panel_context_menu_overlay,
    chat_panel_virtual_window, chat_render_window, chat_todo_overlay,
    compute_visible_message_window_for_chat, is_chat_message_idx_visible, jump_to_bottom_button,
    live_message_meta_fallback, project_user_question_marker_positions, rough_message_heights,
    user_question_nav_overlay, user_question_preview, view,
};
use crate::app::components::chat_panel::virtualizer::ChatVirtualizer;
use crate::app::models::{ChatMessage, ChatRole};
use crate::app::state::UserQuestionMarker;
use crate::app::{App, Message, TodoPanelPlacement};
#[cfg(not(target_arch = "wasm32"))]
use iced::advanced::renderer::Headless;
#[cfg(not(target_arch = "wasm32"))]
use iced::advanced::{Layout, layout, widget::Tree};
#[cfg(not(target_arch = "wasm32"))]
use iced::{Font, Pixels, Size};
use vw_shared::todo::Todo;

fn message(role: ChatRole, content: &str) -> ChatMessage {
    ChatMessage {
        role,
        content: content.to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }
}

fn todo(id: &str) -> Todo {
    Todo {
        id: id.to_string(),
        status: "pending".to_string(),
        content: format!("task {id}"),
        priority: "medium".to_string(),
    }
}

fn question_marker(message_idx: usize) -> UserQuestionMarker {
    UserQuestionMarker { message_idx, preview: format!("question {message_idx}") }
}

#[test]
fn chat_message_column_fills_available_width() {
    let element: iced::Element<'_, Message> = chat_message_column().into();

    assert_eq!(element.as_widget().size().width, iced::Length::Fill);
}

#[test]
fn conversation_shell_borrows_runtime_once() {
    let mut app = App::new().0;
    app.active_session_id = Some("session".to_string());
    app.current_session_runtime_mut().is_requesting = true;
    app.chat = vec![message(ChatRole::User, "one"), message(ChatRole::Assistant, "two")];
    super::performance::reset();

    let _ = view(&app);

    assert_eq!(super::performance::snapshot().runtime_borrows, 1);
    assert_eq!(super::performance::snapshot().runtime_clones, 0);
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn conversation_shell_uses_shared_reading_column_width() {
    const EXPECTED_MAX_WIDTH: f32 = 880.0;
    const HORIZONTAL_PADDING: f32 = 18.0;

    async fn message_column_bounds(viewport_width: f32) -> iced::Rectangle {
        let renderer =
            <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
                .await
                .expect("tiny-skia headless renderer should be available");
        let mut element: iced::Element<'_, Message> =
            iced::widget::container(chat_message_column())
                .width(iced::Length::Fill)
                .center_x(iced::Length::Fill)
                .padding(iced::Padding {
                    top: 0.0,
                    right: HORIZONTAL_PADDING,
                    bottom: 0.0,
                    left: HORIZONTAL_PADDING,
                })
                .into();
        let limits = layout::Limits::new(Size::ZERO, Size::new(viewport_width, 600.0));
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(&mut tree, &renderer, &limits);

        Layout::new(&node)
            .children()
            .next()
            .expect("conversation shell should contain the message column")
            .bounds()
    }

    let wide = message_column_bounds(1_200.0).await;
    assert_eq!(wide.width, EXPECTED_MAX_WIDTH);
    assert_eq!(wide.x, (1_200.0 - EXPECTED_MAX_WIDTH) / 2.0);

    let narrow = message_column_bounds(540.0).await;
    assert_eq!(narrow.width, 540.0 - HORIZONTAL_PADDING * 2.0);
    assert_eq!(narrow.x, HORIZONTAL_PADDING);
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
                .join("../../assets/themes/presets/tahoe-beach-night/background.webp");
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
            app.current_session_runtime_mut().is_requesting = true;
        }
        if matches!(scenario, Scenario::WaitingPermission) {
            app.chat_message_ids = vec![None, Some("msg-permission".to_string())];
            let request = vw_gateway_client::PendingPermissionRequestDto {
                id: "req-permission".to_string(),
                session_id: app.active_session_id.clone().unwrap_or_default(),
                permission: "bash".to_string(),
                patterns: vec![],
                metadata: serde_json::Map::new(),
                always: vec![],
                tool: Some(vw_gateway_client::PendingPermissionToolDto {
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
fn chat_panel_uses_virtual_window_for_render_range() {
    let (mut app, _) = App::new();
    app.chat = (0..120).map(|idx| message(ChatRole::Assistant, &format!("msg {idx}"))).collect();
    app.chat_message_estimated_heights = vec![2000.0; app.chat.len()];
    app.chat_height_index =
        super::height_index::ChatHeightIndex::from_heights(&app.chat_message_estimated_heights);

    let virtual_heights = vec![80.0; app.chat.len()];
    app.chat_virtualizer = ChatVirtualizer::from_estimated_heights(&virtual_heights);
    app.chat_virtualizer.set_viewport_height(180.0);
    app.chat_virtualizer.set_scroll_top_px(4200.0);

    let window = chat_panel_virtual_window(&app);

    assert_eq!(window, app.chat_virtualizer.window());
    assert!(window.render_start_idx > 0);
    assert!(window.render_end_idx < app.chat.len());
    assert!(window.top_spacer_h > 0.0);
    assert!(window.bottom_spacer_h > 0.0);
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

#[test]
fn visible_message_window_clamps_to_available_heights() {
    let heights = vec![40.0, 50.0, 60.0];

    let (start, end) = compute_visible_message_window_for_chat(10, &heights, 0.0, 70.0);

    assert_eq!(start, 0);
    assert!(end <= heights.len());
    assert!(end > start);
}

#[test]
fn visible_message_window_returns_empty_without_messages_or_heights() {
    assert_eq!(compute_visible_message_window_for_chat(0, &[40.0], 0.0, 100.0), (0, 0));
    assert_eq!(compute_visible_message_window_for_chat(2, &[], 0.0, 100.0), (0, 0));
}

#[test]
fn visible_message_idx_requires_positive_non_empty_window() {
    assert!(is_chat_message_idx_visible(1, 0, 2, 200.0));
    assert!(!is_chat_message_idx_visible(2, 0, 2, 200.0));
    assert!(!is_chat_message_idx_visible(1, 0, 2, 0.0));
    assert!(!is_chat_message_idx_visible(1, 2, 2, 200.0));
}

#[test]
fn rough_message_heights_grow_for_rich_messages() {
    let chat = vec![
        message(ChatRole::User, "short"),
        message(ChatRole::Assistant, "<think>x</think>\ntool bash\n{}\nanswer"),
    ];

    let heights = rough_message_heights(&chat);

    assert_eq!(heights.len(), 2);
    assert!(heights[1] > heights[0]);
}

#[test]
fn live_message_meta_fallback_only_applies_to_requesting_tail_chat_roles() {
    assert_eq!(
        live_message_meta_fallback(ChatRole::Assistant, true, true, "gpt"),
        Some("gpt · 刚刚".to_string())
    );
    assert_eq!(
        live_message_meta_fallback(ChatRole::User, true, true, "gpt"),
        Some("gpt · 刚刚".to_string())
    );
    assert_eq!(live_message_meta_fallback(ChatRole::Assistant, false, true, "new-model"), None);
    assert_eq!(live_message_meta_fallback(ChatRole::User, true, false, "new-model"), None);
    assert_eq!(live_message_meta_fallback(ChatRole::System, true, true, "gpt"), None);
}

#[test]
fn active_user_question_returns_first_visible_question() {
    let indices = vec![1, 3, 5];
    assert_eq!(active_user_question_idx(&indices, 0, 4, 200.0), Some(1));
}

#[test]
fn active_user_question_returns_none_when_outside_range() {
    let indices = vec![5, 7];
    assert_eq!(active_user_question_idx(&indices, 0, 4, 200.0), None);
}

#[test]
fn active_user_question_returns_none_with_empty_indices() {
    assert_eq!(active_user_question_idx(&[], 0, 4, 200.0), None);
}

#[test]
fn active_user_question_returns_first_when_multiple_visible() {
    let indices = vec![2, 5, 8];
    assert_eq!(active_user_question_idx(&indices, 2, 9, 200.0), Some(2));
}

#[test]
fn active_user_question_returns_none_with_zero_viewport() {
    let indices = vec![1, 3];
    assert_eq!(active_user_question_idx(&indices, 0, 4, 0.0), None);
}

#[test]
fn chat_render_window_iterates_only_render_slice() {
    let chat = vec![(); 10_000];
    let mut visits = 0;

    for (idx, _) in chat_render_window(&chat, 9_980, 10_000) {
        assert!((9_980..10_000).contains(&idx));
        visits += 1;
    }

    assert_eq!(visits, 20);
}

#[test]
fn question_marker_projection_is_bounded_to_40_and_keeps_active_edges() {
    let markers = (0..10_000).map(question_marker).collect::<Vec<_>>();
    let active_position = 5_000;
    let projected = project_user_question_marker_positions(markers.len(), Some(active_position));

    assert!(projected.len() <= 40);
    assert!(projected.contains(&0));
    assert!(projected.contains(&(markers.len() - 1)));
    assert!(projected.contains(&active_position));
    assert!(projected.contains(&(active_position - 1)));
    assert!(projected.contains(&(active_position + 1)));
}

#[test]
fn chat_panel_widget_tree_size_is_independent_of_total_history() {
    let small_history = vec![(); 100];
    let large_history = vec![(); 10_000];
    let small_rows = chat_render_window(&small_history, 80, 100).count();
    let large_rows = chat_render_window(&large_history, 9_980, 10_000).count();
    let small_markers = project_user_question_marker_positions(100, Some(90));
    let large_markers = project_user_question_marker_positions(10_000, Some(9_990));

    assert_eq!(small_rows, large_rows);
    assert!(small_markers.len() <= 40);
    assert!(large_markers.len() <= 40);
}

#[test]
fn user_question_preview_truncates_long_content() {
    let long = "a".repeat(200);
    assert_eq!(user_question_preview(&long).chars().count(), 101);
}

#[test]
fn user_question_preview_flattens_newlines() {
    let result = user_question_preview("hello\nworld\nfoo");
    assert!(!result.contains('\n'));
    assert!(result.contains(' '));
}

#[test]
fn user_question_preview_returns_placeholder_for_empty() {
    assert_eq!(user_question_preview(""), "空提问");
}

#[test]
fn user_question_preview_short_text_passes_through() {
    assert_eq!(user_question_preview("short question"), "short question");
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
fn chat_panel_context_menu_overlay_builds_closed_and_open_states() {
    let (mut app, _) = App::new();
    keep_element(chat_panel_context_menu_overlay(&app, iced::widget::text("content")));

    app.chat_context_menu_target =
        Some(crate::app::components::chat_panel::utils::chat_context_target_key(0, None));
    app.chat_context_menu_pos = Some((12.0, 24.0));
    keep_element(chat_panel_context_menu_overlay(&app, iced::widget::text("content")));
}

#[test]
fn chat_panel_context_menu_has_a_single_overlay_owner() {
    assert!(include_str!("mod.rs").contains("PointBelowOverlay::new(content, menu)"));
    for source in [
        include_str!("message_view.rs"),
        include_str!("tools/advanced_view.rs"),
        include_str!("tools/apply_patch_view.rs"),
        include_str!("tools/files_view/common_view.rs"),
        include_str!("tools/files_view/write_view.rs"),
        include_str!("tools/text_view.rs"),
        include_str!("tools/todo_views.rs"),
        include_str!("tools/web_view.rs"),
    ] {
        assert!(!source.contains("PointBelowOverlay"));
    }
}

#[test]
fn jump_button_and_question_overlay_cover_hidden_and_visible_states() {
    let (mut app, _) = App::new();
    keep_element(jump_to_bottom_button(&app));
    keep_element(user_question_nav_overlay(&app, &[], 0, 0));

    app.chat_auto_scroll = false;
    app.chat_scroll_viewport_h = 200.0;
    app.chat = vec![message(ChatRole::User, "first\nquestion")];
    app.rebuild_active_session_message_meta();
    keep_element(jump_to_bottom_button(&app));
    keep_element(user_question_nav_overlay(
        &app,
        &app.active_session_view_state.user_question_markers,
        0,
        1,
    ));
}

#[test]
fn chat_todo_overlay_builds_only_for_matching_top_right_session_with_items() {
    let (mut app, _) = App::new();
    app.active_session_id = Some("active".to_string());
    app.chat_todo_session_id = Some("other".to_string());
    app.chat_todo_items = vec![todo("1")];

    keep_element(chat_todo_overlay(&app));

    app.chat_todo_session_id = Some("active".to_string());
    app.chat_todo_placement = TodoPanelPlacement::InputBottom;
    keep_element(chat_todo_overlay(&app));

    app.chat_todo_placement = TodoPanelPlacement::ChatTopRight;
    keep_element(chat_todo_overlay(&app));
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn virtual_content_layout_matches_height_index_at_all_scroll_positions() {
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("headless renderer");
    let mut virtualizer = ChatVirtualizer::from_estimated_heights(&[100.0; 80]);
    virtualizer.set_viewport_height(300.0);
    for offset in [0.0, 3000.0, 9000.0, 0.0] {
        virtualizer.set_scroll_top_px(offset);
        let window = virtualizer.window();
        let mut messages = chat_message_column();
        for _ in window.render_start_idx..window.render_end_idx {
            messages = messages.push(iced::widget::Space::new().height(100.0));
        }
        let mut element: iced::Element<'_, Message> = super::conversation_reading_column(
            super::chat_virtual_content(messages, window.top_spacer_h, window.bottom_spacer_h),
            iced::Padding { top: 0.0, right: 18.0, bottom: 0.0, left: 18.0 },
        )
        .into();
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, Size::new(800.0, f32::INFINITY)),
        );
        assert!((node.size().height - virtualizer.total_height_px()).abs() < 0.01);
    }
}
