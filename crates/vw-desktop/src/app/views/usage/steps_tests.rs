use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
use iced::{Font, Pixels, Rectangle, Size, Theme};

use crate::app::App;
use crate::app::models::{ChatSession, ChatSessionStep, TokenUsage};

fn usage_data_with_steps(steps: Vec<ChatSessionStep>) -> super::UsageData {
    super::UsageData {
        total_tokens: 0,
        session: Some(ChatSession {
            id: "session-1".to_string(),
            title: "Session".to_string(),
            messages: Vec::new(),
            message_ids: Vec::new(),
            calls: Vec::new(),
            steps,
            created_ms: 1,
            updated_ms: 2,
        }),
        session_id: "session-1".to_string(),
        message_count: 0,
        call_count: 0,
        session_title: "Session".to_string(),
        last_step_input_tokens: 0,
        last_step_output_tokens: 0,
        last_step_cached_tokens: 0,
        last_step_reasoning_tokens: 0,
        last_step_total_tokens: 0,
        user_msgs: 0,
        assistant_msgs: 0,
        system_msgs: 0,
        tool_msgs: 0,
    }
}

fn step(index: u32, finished_ms: Option<u64>) -> ChatSessionStep {
    ChatSessionStep {
        index,
        turn_id: None,
        started_ms: 1_700_000_000_000 + u64::from(index) * 1_000,
        finished_ms,
        start_snapshot_path: Some(format!("/tmp/session/{index}-start.snap")),
        finish_snapshot_path: Some(format!("/tmp/session/{index}-finish.snap")),
        usage: TokenUsage {
            input_tokens: 10,
            output_tokens: 20,
            cached_tokens: 3,
            reasoning_tokens: 4,
        },
        cost_usd: Some(0.012345),
        finish_reason: Some("stop".to_string()),
        model: Some("model-a".to_string()),
        tool_timings: Vec::new(),
    }
}

#[test]
fn seg_color_covers_all_segments() {
    for seg in [
        super::BarSeg::User,
        super::BarSeg::Assistant,
        super::BarSeg::Tool,
        super::BarSeg::Other,
        super::BarSeg::Prompt,
        super::BarSeg::Answer,
    ] {
        let color = super::seg_color(&Theme::Dark, seg);
        assert!(color.a > 0.0);
    }
}

#[test]
fn stacked_bar_handles_empty_zero_and_weighted_values() {
    let _empty = super::stacked_bar(&[]);
    let _zero = super::stacked_bar(&[(0, super::BarSeg::User), (0, super::BarSeg::Tool)]);
    let _weighted = super::stacked_bar(&[(1, super::BarSeg::Prompt), (999, super::BarSeg::Answer)]);
}

#[test]
fn stacked_bar_handles_minimum_portion_overflow() {
    let values = vec![(1, super::BarSeg::Tool); 2_001];

    let _bar = super::stacked_bar(&values);
}

#[test]
fn stacked_bar_skips_zero_width_segments_when_total_is_nonzero() {
    let bar = super::stacked_bar(&[(0, super::BarSeg::User), (5, super::BarSeg::Assistant)]);
    let tree = Tree::new(bar.as_widget());

    assert_eq!(tree.children.len(), 1);
}

#[test]
fn legend_item_handles_zero_and_nonzero_totals() {
    let _zero = super::legend_item("输入", 0, 0, super::BarSeg::Prompt);
    let _half = super::legend_item("输出", 25, 50, super::BarSeg::Answer);
}

#[test]
fn build_steps_panel_handles_empty_steps() {
    let (app, _task) = App::new();
    let data = usage_data_with_steps(Vec::new());

    let _panel = super::build_steps_panel(&app, &data);
}

#[test]
fn build_steps_panel_handles_missing_session() {
    let (app, _task) = App::new();
    let mut data = usage_data_with_steps(Vec::new());
    data.session = None;

    let _panel = super::build_steps_panel(&app, &data);
}

#[test]
fn build_steps_panel_handles_collapsed_and_expanded_steps() {
    let (mut app, _task) = App::new();
    app.usage_step_expanded.insert(1);
    let data = usage_data_with_steps(vec![step(1, Some(1_700_000_002_000)), step(2, None)]);

    let _panel = super::build_steps_panel(&app, &data);
}

#[test]
fn build_steps_panel_handles_expanded_step_without_optional_fields() {
    let (mut app, _task) = App::new();
    app.usage_step_expanded.insert(3);
    let mut s = step(3, None);
    s.start_snapshot_path = None;
    s.finish_snapshot_path = None;
    s.cost_usd = None;
    s.finish_reason = None;
    s.model = None;
    s.usage = TokenUsage::default();
    let data = usage_data_with_steps(vec![s]);

    let _panel = super::build_steps_panel(&app, &data);
}

#[test]
fn raw_payload_status_reports_loaded_loading_error_and_missing_states() {
    let (mut app, _task) = App::new();
    app.usage_step_raw_payloads.insert(
        ("session-1".to_string(), String::new(), 1),
        Ok(Some(serde_json::json!({
            "messages": [{"role": "user", "content": "hello"}],
            "output": {"tool_calls": [{"id": "call-1"}]}
        }))),
    );
    app.usage_step_raw_payloads
        .insert(
            ("session-1".to_string(), String::new(), 2),
            Err("gateway request failed".to_string()),
        );
    app.usage_step_raw_loading.insert(("session-1".to_string(), String::new(), 3));

    assert!(super::raw_payload_status(&app, "session-1", 1).contains("历史消息 1 条"));
    assert!(super::raw_payload_status(&app, "session-1", 2).contains("加载失败"));
    assert!(super::raw_payload_status(&app, "session-1", 3).contains("加载中"));
    assert!(super::raw_payload_status(&app, "session-1", 4).contains("等待加载"));
}

#[test]
fn raw_payload_status_reports_absent_record_state() {
    let (mut app, _task) = App::new();
    app.usage_step_raw_payloads.insert(("session-1".to_string(), String::new(), 5), Ok(None));

    assert!(super::raw_payload_status(&app, "session-1", 5).contains("暂无记录"));
}

#[test]
fn raw_payload_status_prefers_loading_state_over_cached_payload() {
    let (mut app, _task) = App::new();
    let key = ("session-1".to_string(), String::new(), 6);
    app.usage_step_raw_payloads
        .insert(key.clone(), Ok(Some(serde_json::json!({"messages": [{"role": "user"}]}))));
    app.usage_step_raw_loading.insert(key);

    assert_eq!(super::raw_payload_status(&app, "session-1", 6), "原始记录：加载中...");
}

#[test]
fn raw_payload_summary_counts_missing_and_present_fields() {
    let empty = serde_json::json!({});
    let populated = serde_json::json!({
        "messages": [{"role": "user"}, {"role": "assistant"}],
        "output": {"tool_calls": [{"id": "call-1"}, {"id": "call-2"}]}
    });

    assert_eq!(super::raw_payload_summary(&empty), "历史消息 0 条 · 工具调用 0 条");
    assert_eq!(super::raw_payload_summary(&populated), "历史消息 2 条 · 工具调用 2 条");
}

#[test]
fn raw_payload_panel_builds_for_loaded_and_missing_payload() {
    let (mut app, _task) = App::new();
    app.usage_step_raw_payloads.insert(
        ("session-1".to_string(), String::new(), 1),
        Ok(Some(serde_json::json!({
            "messages": [{"role": "user", "content": "hello"}],
            "output": {"tool_calls": []}
        }))),
    );

    let _loaded = super::raw_payload_panel(&app, "session-1", 1);
    let _missing = super::raw_payload_panel(&app, "session-1", 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn expanded_steps_panel_draws_dark_theme_styles() {
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let (mut app, _task) = App::new();
    app.usage_step_expanded.insert(1);
    let data = usage_data_with_steps(vec![step(1, Some(1_700_000_002_000))]);
    let mut element = super::build_steps_panel(&app, &data);
    let bounds = Size::new(800.0, 520.0);
    let limits = layout::Limits::new(Size::ZERO, bounds);
    let mut tree = Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(&mut tree, &mut renderer, &limits);

    element.as_widget().draw(
        &tree,
        &mut renderer,
        &Theme::Dark,
        &renderer::Style::default(),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &Rectangle::with_size(bounds),
    );
}
