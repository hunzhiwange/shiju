use super::global_status::{chat_global_status, format_elapsed, think_pulse_color, think_pulse_geometry, think_pulse_glyph, think_pulse_size, tokens_label};
use session_ui_types::{ChatMessage, ChatRole, TokenUsage};
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
    let content =
        format!("tool read\n{}", serde_json::json!({ "status": "running", "input": input }));

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
fn thinking_pulse_cycles_original_shapes_and_breathes_more_slowly() {
    assert_eq!(think_pulse_glyph(0), "✳");
    assert_eq!(think_pulse_glyph(11), "✳");
    assert_eq!(think_pulse_glyph(12), "✶");
    assert_eq!(think_pulse_glyph(24), "✷");
    assert_eq!(think_pulse_glyph(36), "❋");
    assert_eq!(think_pulse_glyph(48), "✳");

    assert!((think_pulse_size(0) - 14.0).abs() < 0.001);
    assert!((think_pulse_size(12) - 44.0).abs() < 0.001);
    assert!((think_pulse_size(24) - 14.0).abs() < 0.001);
    assert_eq!(think_pulse_size(0), think_pulse_size(1));
    assert_eq!(think_pulse_color(0), think_pulse_color(48));

    for frame in 0..48 {
        assert!((14.0..=44.0).contains(&think_pulse_size(frame)), "frame {frame}");
        assert_eq!(think_pulse_geometry(frame).0, 44.0, "frame {frame}");
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
fn tokens_label_ignores_negative_components() {
    let usage = TokenUsage {
        input_tokens: -20,
        output_tokens: 25,
        reasoning_tokens: -5,
        cached_tokens: 99,
    };

    assert_eq!(tokens_label(&usage).as_deref(), Some("25 tokens"));
}
