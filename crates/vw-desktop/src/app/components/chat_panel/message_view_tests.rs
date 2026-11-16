//! 验证聊天消息视图构建。
//! 测试聚焦消息类型与渲染输出的映射，防止 UI 回归影响可读性。

use super::message_view::{message_surface_contract, message_surface_style, message_view, parse_markdown_blocks, should_use_user_message_editor};
use crate::app::App;
use crate::app::models::{ChatRenderCacheEntry, ChatRole, ThinkTiming};
use iced::advanced::renderer::Headless;
use iced::advanced::widget::Tree;
use iced::advanced::{Layout, layout, mouse, renderer};
use iced::widget::{column, container};
use iced::{Background, Color, Font, Length, Pixels, Rectangle, Size, Theme};
use serde_json::Map;
use std::borrow::Cow;
use session_ui_types::ProcessingTiming;

#[test]
fn user_message_falls_back_to_source_text_while_selectable_editor_is_empty() {
    let (mut app, _) = App::new();
    app.chat_message_editors.push(iced::widget::text_editor::Content::new());

    assert!(!should_use_user_message_editor(&app, 0, "刚发送的消息", true));

    app.chat_message_editors[0] = iced::widget::text_editor::Content::with_text("刚发送的消息");
    assert!(should_use_user_message_editor(&app, 0, "刚发送的消息", true));
    assert!(!should_use_user_message_editor(&app, 0, "刚发送的消息", false));
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn selectable_user_message_fills_available_width() {
    const CONTENT: &str = "这是一条纯文本用户消息，用来确认可选择文本不会被压成竖直长条。";

    let (mut app, _) = App::new();
    app.chat_message_editors.push(iced::widget::text_editor::Content::with_text(CONTENT));
    let cache = ChatRenderCacheEntry::default();
    let mut element = message_view(&app, 0, ChatRole::User, CONTENT, &[], None, None, &cache, true);
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer");
    let limits = layout::Limits::new(Size::ZERO, Size::new(1_000.0, 1_200.0));
    let mut tree = Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(&mut tree, &renderer, &limits);
    let bubble_shell =
        Layout::new(&node).children().next().expect("message column should contain a bubble shell");
    let bubble =
        bubble_shell.children().next().expect("bubble shell should contain the user bubble");

    assert!(node.bounds().height < 240.0, "user bubble was unexpectedly tall: {:?}", node.bounds());
    assert_eq!(bubble.bounds().width, node.bounds().width);
}

#[test]
fn message_visual_contract_keeps_assistant_flat_and_user_full_width() {
    let (app, _) = App::new();
    let cache = ChatRenderCacheEntry::default();
    let user = message_view(
        &app,
        0,
        ChatRole::User,
        "请检查 the visual hierarchy\n\n[DOCUMENT:/tmp/界面-spec.pdf]",
        &[],
        None,
        Some(Cow::Borrowed("You · now")),
        &cache,
        true,
    );
    let assistant = message_view(
        &app,
        1,
        ChatRole::Assistant,
        "# 结果\n\n正文保持平面。",
        &[],
        None,
        Some(Cow::Borrowed("Assistant · now")),
        &cache,
        true,
    );
    std::hint::black_box((user, assistant));

    let user = message_surface_contract(true);
    let assistant = message_surface_contract(false);
    assert!(matches!(user.width, Length::Fill));
    assert_eq!(user.max_width, None);
    assert_eq!(user.padding, iced::Padding::from([8, 14]));
    assert_eq!(user.corner_radius, 12.0);
    assert!(user.has_background);
    assert!(matches!(assistant.width, Length::Fill));
    assert_eq!(assistant.max_width, None);
    assert!(!assistant.has_background);
    assert!(message_surface_style(&Theme::Light, true, false).background.is_some());
    assert!(message_surface_style(&Theme::Dark, false, false).background.is_none());
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn message_visual_contract_renders_complex_content_in_light_and_dark_themes() {
    const USER_CONTENT: &str = "请检查 the visual hierarchy，确认纯文本用户消息保持可见且紧凑。";
    const MARKDOWN_CONTENT: &str = concat!(
        "# 中文标题与 English heading\n\n",
        "这是一段较长的正文，用来检查中英文混排、行高与消息阅读节奏。",
        " The paragraph stays readable without turning the assistant response into a card.\n\n",
        "- 第一项\n  - nested item\n    - 更深一层\n\n",
        "> 引用文本应该保持克制的左侧提示条。\n\n",
        "```rust\n// 中文注释验证默认聊天字体\nlet value = 1;\n```",
    );
    const TOOL_CONTENT: &str = concat!(
        "<think>正在核对权限与附件展示</think>\n",
        "tool read\n",
        "{\"tool_call_id\":\"call-visual\",\"input\":\"{\\\"filePath\\\":\\\"src/中文路径.rs\\\"}\",\"status\":\"waiting\"}\n",
        "工具活动之后的正文。",
    );

    for (theme, screenshot_name) in
        [(Theme::Light, "message-light.png"), (Theme::Dark, "message-dark.png")]
    {
        let (mut app, _) = App::new();
        app.app_theme = theme.clone();
        app.chat_message_editors.push(iced::widget::text_editor::Content::with_text(USER_CONTENT));
        app.chat_message_ids = vec![None, None, Some("msg-visual".to_string())];
        app.chat_markdown_blocks.insert(1_u64 << 32, parse_markdown_blocks(MARKDOWN_CONTENT));
        let request = gateway_interaction::PendingPermissionRequestDto {
            id: "req-visual".to_string(),
            session_id: "session-visual".to_string(),
            permission: "read".to_string(),
            patterns: vec![],
            metadata: Map::new(),
            always: vec![],
            tool: Some(gateway_interaction::PendingPermissionToolDto {
                message_id: "msg-visual".to_string(),
                call_id: "call-visual".to_string(),
            }),
        };
        app.permission_modal_request_id = Some(request.id.clone());
        app.permission_modal_request = Some(request.clone());
        app.permission_modal_requests = vec![request];

        let cache = ChatRenderCacheEntry::default();
        let processing = ProcessingTiming {
            started_ms: 1_000,
            completed_ms: Some(26_000),
            processing_elapsed_ms: Some(25_000),
            llm_elapsed_ms: Some(20_000),
        };
        let think_timing = [ThinkTiming { start_ms: 1, end_ms: Some(2), last_update_ms: 2 }];
        let messages = column![
            message_view(
                &app,
                0,
                ChatRole::User,
                USER_CONTENT,
                &[],
                None,
                Some(Cow::Borrowed("You · now")),
                &cache,
                true,
            ),
            message_view(
                &app,
                1,
                ChatRole::Assistant,
                MARKDOWN_CONTENT,
                &[],
                Some(&processing),
                Some(Cow::Borrowed("Assistant · now")),
                &cache,
                true,
            ),
            message_view(
                &app,
                2,
                ChatRole::Assistant,
                TOOL_CONTENT,
                &think_timing,
                None,
                Some(Cow::Borrowed("Assistant · permission pending")),
                &cache,
                true,
            ),
        ]
        .spacing(24);
        let mut element: iced::Element<'_, crate::app::Message> = container(messages)
            .padding(32)
            .width(Length::Fill)
            .style(|theme: &Theme| iced::widget::container::Style {
                background: Some(Background::Color(theme.palette().background)),
                ..Default::default()
            })
            .into();
        let mut renderer =
            <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
                .await
                .expect("tiny-skia headless renderer");
        let bounds = Size::new(1_000.0, 1_200.0);
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, bounds),
        );
        assert!(node.bounds().width <= bounds.width);
        assert!(node.bounds().height <= bounds.height);
        let viewport = Rectangle::with_size(bounds);
        element.as_widget().draw(
            &tree,
            &mut renderer,
            &theme,
            &renderer::Style::default(),
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &viewport,
        );
        let pixels = renderer.screenshot(Size::new(1_000, 1_200), 1.0, Color::TRANSPARENT);
        assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));

        if let Ok(directory) = std::env::var("VW_MESSAGE_VISUAL_SCREENSHOT_DIR") {
            image::save_buffer(
                std::path::Path::new(&directory).join(screenshot_name),
                &pixels,
                1_000,
                1_200,
                image::ColorType::Rgba8,
            )
            .expect("save message visual screenshot");
        }
    }
}

#[test]
fn message_view_builds_user_message_with_metadata_and_attachment() {
    let app = App::new().0;
    let content = "please review\n\n[DOCUMENT:/tmp/spec.pdf]";
    let cache = ChatRenderCacheEntry::default();

    let view = message_view(
        &app,
        0,
        ChatRole::User,
        content,
        &[],
        None,
        Some(Cow::Borrowed("You · now")),
        &cache,
        true,
    );

    std::hint::black_box(view);
}

#[test]
fn message_view_builds_assistant_special_blocks_without_metadata() {
    let app = App::new().0;
    let content = "<think>inspect</think>\ntool read\n{\"status\":\"completed\",\"input\":\"{\\\"filePath\\\":\\\"src/lib.rs\\\"}\"}\nanswer";
    let cache = ChatRenderCacheEntry::default();

    let view = message_view(&app, 0, ChatRole::Assistant, content, &[], None, None, &cache, false);

    std::hint::black_box(view);
}

#[test]
fn message_view_builds_tool_role_with_metadata() {
    let app = App::new().0;
    let content = "tool shell\n{\"status\":\"completed\",\"input\":\"pwd\",\"output\":\"/tmp\"}";
    let cache = ChatRenderCacheEntry::default();

    let view = message_view(
        &app,
        0,
        ChatRole::Tool,
        content,
        &[],
        None,
        Some(Cow::Borrowed("Tool")),
        &cache,
        true,
    );

    std::hint::black_box(view);
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn long_user_message_expands_and_collapses_with_attachments() {
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer");
    let limits = layout::Limits::new(Size::ZERO, Size::new(700.0, 5000.0));
    for attachment in ["", "\n[DOCUMENT:/tmp/reference.pdf]"] {
        let content = format!("{}{attachment}", "中文消息与日志 output\n".repeat(60));
        let (mut app, _) = App::new();
        app.chat_message_editors.push(iced::widget::text_editor::Content::with_text(&content));
        let cache = ChatRenderCacheEntry::default();
        let mut heights = Vec::new();
        for expanded in [false, true, false] {
            if expanded {
                app.chat_message_expanded.insert(0);
            } else {
                app.chat_message_expanded.remove(&0);
            }
            let mut element =
                message_view(&app, 0, ChatRole::User, &content, &[], None, None, &cache, false);
            let mut tree = Tree::new(element.as_widget());
            let node = element.as_widget_mut().layout(&mut tree, &renderer, &limits);
            heights.push(node.bounds().height);
            for theme in [Theme::Light, Theme::Dark] {
                element.as_widget().draw(
                    &tree,
                    &mut renderer,
                    &theme,
                    &renderer::Style::default(),
                    Layout::new(&node),
                    mouse::Cursor::Unavailable,
                    &Rectangle::with_size(Size::new(700.0, 5000.0)),
                );
            }
        }
        assert!(heights[1] > heights[0] + 200.0, "{heights:?}");
        assert_eq!(heights[0], heights[2]);
    }
}
