use super::global_status::{
    COMPACT_ACTIVITY_ICON_SIZE, COMPACT_ACTIVITY_SLOT_SIZE, chat_global_status,
    chat_global_status_view, compact_activity_geometry, compact_activity_glyph, format_elapsed,
    think_pulse_glyph, think_pulse_opacity, tokens_label,
};
use crate::app::App;
use crate::app::models::{ChatMessage, ChatRole, TokenUsage};
#[cfg(not(target_arch = "wasm32"))]
use iced::advanced::{Layout, layout, renderer::Headless, widget::Tree};
#[cfg(not(target_arch = "wasm32"))]
use iced::{Font, Pixels, Size};

fn assistant(content: &str) -> ChatMessage {
    ChatMessage {
        role: ChatRole::Assistant,
        content: content.to_string(),
        model: None,
        think_timing: vec![],
        processing_timing: None,
    }
}

fn user(content: &str) -> ChatMessage {
    ChatMessage {
        role: ChatRole::User,
        content: content.to_string(),
        model: None,
        think_timing: vec![],
        processing_timing: None,
    }
}

#[test]
fn running_read_tool_becomes_latest_status() {
    let chat = vec![assistant(
        "tool read\n{\"status\":\"running\",\"input\":\"{\\\"filePath\\\":\\\"src/main.rs\\\"}\"}",
    )];

    let status = chat_global_status(&chat, true);

    let status = status.expect("running status should render");
    assert_eq!(status.action, "正在读取");
    assert_eq!(status.detail, "src/main.rs");
}

#[test]
fn running_tool_status_keeps_full_detail_for_layout_clipping() {
    let long_path = format!("/workspace/{}target.rs", "deeply_nested_directory/".repeat(8));
    let input = serde_json::json!({ "filePath": long_path }).to_string();
    let content = format!(
        "tool read\n{}",
        serde_json::json!({ "status": "running", "input": input })
    );

    let status = chat_global_status(&[assistant(&content)], true).expect("running read status");

    assert_eq!(status.detail, long_path);
    assert!(status.detail.chars().count() > 80);
}

#[test]
fn latest_tool_replaces_previous_status() {
    let chat = vec![assistant(
        "tool read\n{\"status\":\"completed\",\"input\":\"{\\\"filePath\\\":\\\"old.rs\\\"}\"}\n\
         tool shell\n{\"status\":\"running\",\"input\":\"{\\\"command\\\":\\\"cargo clippy\\\"}\"}",
    )];

    let status = chat_global_status(&chat, true);

    let status = status.expect("latest running status should render");
    assert_eq!(status.action, "正在运行");
    assert_eq!(status.detail, "cargo clippy");
}

#[test]
fn open_think_without_tool_reports_thinking() {
    let chat = vec![assistant("<think>第一行\n第二行\n第三行")];

    let status = chat_global_status(&chat, true);

    let status = status.expect("thinking status should render");
    assert_eq!(status.action, "思考中");
    assert_eq!(status.detail, "第二行\n第三行");
}

#[test]
fn completed_tool_without_request_hides_status() {
    let chat = vec![assistant(
        "tool shell\n{\"status\":\"completed\",\"input\":\"{\\\"command\\\":\\\"cargo clippy\\\"}\"}",
    )];

    assert_eq!(chat_global_status(&chat, false), None);
}

#[test]
fn failed_tool_hides_status_after_request_finishes() {
    let chat = vec![assistant(
        "tool shell\n{\"status\":\"error\",\"input\":\"{\\\"command\\\":\\\"bash fetch_shipment.sh\\\"}\"}",
    )];

    assert_eq!(chat_global_status(&chat, false), None);
}

#[test]
fn idle_chat_hides_status() {
    assert_eq!(chat_global_status(&[], false), None);
}

#[test]
fn format_elapsed_switches_to_minutes_after_a_minute() {
    assert_eq!(format_elapsed(0), "0秒");
    assert_eq!(format_elapsed(43), "43秒");
    assert_eq!(format_elapsed(60), "1分 0秒");
    assert_eq!(format_elapsed(176), "2分 56秒");
}

#[test]
fn global_status_indicator_uses_shared_spinner_frames() {
    for frame in 0..24 {
        assert_eq!(
            compact_activity_glyph(frame),
            crate::app::components::status_animation::spinner_frame(frame)
        );
    }
}

#[test]
fn thinking_pulse_cycles_shapes_and_breathes() {
    assert_eq!(think_pulse_glyph(0), "✳");
    assert_eq!(think_pulse_glyph(6), "✶");
    assert_eq!(think_pulse_glyph(12), "✷");
    assert_eq!(think_pulse_glyph(18), "❋");
    assert_eq!(think_pulse_glyph(24), "✳");

    assert!((think_pulse_opacity(0) - 0.35).abs() < 0.001);
    assert!((think_pulse_opacity(6) - 1.0).abs() < 0.001);
    for frame in 0..30 {
        assert!((0.35..=1.0).contains(&think_pulse_opacity(frame)));
    }
}

#[test]
fn global_status_indicator_keeps_fixed_geometry_across_frames() {
    let mut app = App::new().0;
    app.current_session_runtime_mut().is_requesting = true;
    app.chat.push(assistant(
        "tool read\n{\"status\":\"running\",\"input\":\"{\\\"filePath\\\":\\\"含中文/很长的工具摘要.rs\\\"}\"}",
    ));
    app.usage = TokenUsage { input_tokens: 1200, output_tokens: 34, ..Default::default() };

    for frame in 0..30 {
        app.status_animation_frame = frame;
        std::hint::black_box(chat_global_status_view(&app));
        assert_eq!(
            compact_activity_geometry(frame),
            (COMPACT_ACTIVITY_SLOT_SIZE, COMPACT_ACTIVITY_ICON_SIZE),
            "frame {frame}"
        );
    }
}

#[test]
fn tokens_label_summarizes_total_usage() {
    assert_eq!(tokens_label(&TokenUsage::default()), None);

    let small = TokenUsage { input_tokens: 120, output_tokens: 80, ..Default::default() };
    assert_eq!(tokens_label(&small).as_deref(), Some("200 tokens"));

    let boundary = TokenUsage { input_tokens: 1000, ..Default::default() };
    assert_eq!(tokens_label(&boundary).as_deref(), Some("1.0k tokens"));

    let large = TokenUsage {
        input_tokens: 3000,
        output_tokens: 200,
        reasoning_tokens: 0,
        cached_tokens: 0,
    };
    assert_eq!(tokens_label(&large).as_deref(), Some("3.2k tokens"));
}

#[test]
fn closed_think_keeps_status_until_request_finishes() {
    let chat = vec![assistant("<think>分析现有模块</think>")];

    let status = chat_global_status(&chat, true).expect("requesting think status should render");
    assert_eq!(status.action, "思考中");
    assert_eq!(status.detail, "分析现有模块");
    assert_eq!(chat_global_status(&chat, false), None);
}

#[test]
fn requesting_chat_without_assistant_hides_status() {
    assert_eq!(chat_global_status(&[user("hello")], true), None);
}

#[test]
fn failed_tool_reports_failure_while_request_is_active() {
    let chat = vec![assistant(
        "tool shell\n{\"status\":\"error\",\"input\":\"{\\\"command\\\":\\\"cargo check\\\"}\"}",
    )];

    let status = chat_global_status(&chat, true).expect("active failure should render");

    assert_eq!(status.action, "运行失败");
    assert_eq!(status.detail, "cargo check");
}

#[test]
fn global_status_distinguishes_waiting_running_failed_and_cancelled() {
    let cases = [
        ("waiting", "等待运行"),
        ("queued", "等待运行"),
        ("running", "正在运行"),
        ("failed", "运行失败"),
        ("denied", "运行失败"),
        ("cancelled", "运行已取消"),
    ];

    for (raw_status, expected_action) in cases {
        let chat = vec![assistant(&format!(
            "tool shell\n{{\"status\":\"{raw_status}\",\"input\":\"{{\\\"command\\\":\\\"cargo check\\\"}}\"}}"
        ))];

        let status = chat_global_status(&chat, true)
            .unwrap_or_else(|| panic!("{raw_status} status should render during the request"));

        assert_eq!(status.action, expected_action, "raw status: {raw_status}");
        assert_eq!(status.detail, "cargo check");
        assert_eq!(chat_global_status(&chat, false), None);
    }
}

#[test]
fn statusless_tool_is_running_only_during_active_request() {
    let chat = vec![assistant("tool read\n{\"input\":\"{\\\"filePath\\\":\\\"src/lib.rs\\\"}\"}")];

    let status = chat_global_status(&chat, true).expect("active tool should render");

    assert_eq!(status.action, "正在读取");
    assert_eq!(status.detail, "src/lib.rs");
    assert_eq!(chat_global_status(&chat, false), None);
}

#[test]
fn waiting_tool_status_is_visible_during_request() {
    let chat = vec![assistant(
        "tool shell\n{\"status\":\"queued\",\"input\":\"{\\\"command\\\":\\\"pwd\\\"}\"}",
    )];

    let status = chat_global_status(&chat, true).expect("queued tool should render");
    assert_eq!(status.action, "等待运行");
    assert_eq!(status.detail, "pwd");
}

#[test]
fn status_uses_latest_assistant_even_when_user_message_is_newer() {
    let chat = vec![
        assistant(
            "tool shell\n{\"status\":\"running\",\"input\":\"{\\\"command\\\":\\\"cargo check\\\"}\"}",
        ),
        user("please continue"),
    ];

    let status = chat_global_status(&chat, true).expect("assistant tool should remain visible");

    assert_eq!(status.action, "正在运行");
    assert_eq!(status.detail, "cargo check");
}

#[test]
fn multiline_tool_json_block_parses_until_complete_value() {
    let chat = vec![assistant(
        "prefix\n\
         tool shell\n{\n  \"status\": \"running\",\n  \"input\": \"{\\\"command\\\":\\\"cargo test\\\"}\"\n}\n\
         answer after tool",
    )];

    let status = chat_global_status(&chat, true).expect("multiline tool should parse");

    assert_eq!(status.action, "正在运行");
    assert_eq!(status.detail, "cargo test");
}

#[test]
fn global_status_view_builds_for_hidden_and_visible_states() {
    let mut app = App::new().0;

    let _hidden = chat_global_status_view(&app);
    drop(_hidden);

    app.current_session_runtime_mut().is_requesting = true;
    app.current_session_runtime_mut().request_started_ms = 1;
    app.chat.push(assistant(
        "tool read\n{\"status\":\"running\",\"input\":\"{\\\"filePath\\\":\\\"src/app.rs\\\"}\"}",
    ));
    app.usage = TokenUsage { input_tokens: 1200, output_tokens: 34, ..Default::default() };
    app.status_animation_frame = 7;

    let _visible = chat_global_status_view(&app);
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn global_status_detail_fills_remaining_width() {
    async fn detail_width(viewport_width: f32) -> f32 {
        let renderer =
            <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
                .await
                .expect("tiny-skia headless renderer should be available");
        let mut app = App::new().0;
        app.current_session_runtime_mut().is_requesting = true;
        app.chat.push(assistant(&format!("<think>{}</think>", "很长的思考内容".repeat(40))));

        let mut element = chat_global_status_view(&app);
        let limits = layout::Limits::new(Size::ZERO, Size::new(viewport_width, 100.0));
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(&mut tree, &renderer, &limits);
        let row = Layout::new(&node).children().next().expect("status row");

        row.children().nth(1).expect("status detail").bounds().width
    }

    let narrow = detail_width(360.0).await;
    let wide = detail_width(800.0).await;

    assert!(wide > narrow + 400.0, "narrow={narrow}, wide={wide}");
}

#[test]
fn tokens_label_ignores_negative_components() {
    let usage = TokenUsage {
        input_tokens: -20,
        output_tokens: 25,
        reasoning_tokens: -5,
        cached_tokens: 99,
    };

    assert_eq!(tokens_label(&usage).as_deref(), Some("25 tokens"));
}
