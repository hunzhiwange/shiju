use super::global_status::{
    chat_global_status, chat_global_status_view, format_elapsed, pulse_glyph, pulse_size,
    tokens_label,
};
use crate::app::App;
use crate::app::models::{ChatMessage, ChatRole, TokenUsage};

fn assistant(content: &str) -> ChatMessage {
    ChatMessage { role: ChatRole::Assistant, content: content.to_string(), model: None, think_timing: vec![], processing_timing: None }
}

fn user(content: &str) -> ChatMessage {
    ChatMessage { role: ChatRole::User, content: content.to_string(), model: None, think_timing: vec![], processing_timing: None }
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
    assert_eq!(status.action, "正在思考");
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
fn pulse_glyph_cycles_four_shapes_every_half_second() {
    // 每 6 帧（约半秒）切换一次，循环四种形状后回到起点。
    assert_eq!(pulse_glyph(0), "✳");
    assert_eq!(pulse_glyph(5), "✳");
    assert_eq!(pulse_glyph(6), "✶");
    assert_eq!(pulse_glyph(12), "✷");
    assert_eq!(pulse_glyph(18), "❋");
    assert_eq!(pulse_glyph(24), "✳");
}

#[test]
fn pulse_size_breathes_within_bounds() {
    // 呼吸周期 12 帧：起点最小，第 6 帧放到最大，再回到最小。
    assert!((pulse_size(0) - 14.0).abs() < 0.001);
    assert!((pulse_size(6) - 34.0).abs() < 0.001);
    for frame in 0..30 {
        let size = pulse_size(frame);
        assert!((14.0..=34.0).contains(&size), "frame {frame} -> {size}");
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
    assert_eq!(status.action, "正在思考");
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
fn statusless_tool_is_running_only_during_active_request() {
    let chat = vec![assistant("tool read\n{\"input\":\"{\\\"filePath\\\":\\\"src/lib.rs\\\"}\"}")];

    let status = chat_global_status(&chat, true).expect("active tool should render");

    assert_eq!(status.action, "正在读取");
    assert_eq!(status.detail, "src/lib.rs");
    assert_eq!(chat_global_status(&chat, false), None);
}

#[test]
fn unknown_tool_status_hides_status() {
    let chat = vec![assistant(
        "tool shell\n{\"status\":\"queued\",\"input\":\"{\\\"command\\\":\\\"pwd\\\"}\"}",
    )];

    assert_eq!(chat_global_status(&chat, true), None);
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
