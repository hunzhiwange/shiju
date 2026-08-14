use iced::Font;
use iced::advanced::widget::Tree;
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{button, text_input};
use serde_json::json;

use super::details::{
    DETAILS_SCROLLBAR_WIDTH, build_trajectory_details, detail_payload_font,
    detail_payload_wrapping, details_scroll_direction, input_preview_text, trajectory_details,
};
use super::{
    MetricAvailability, TrajectoryDetailsAction, TrajectoryDetailsState, TrajectoryDetailsTab,
    TrajectoryMetrics, TrajectoryRecord, TrajectoryRecordDetail, TrajectoryRecordKind,
    TrajectoryUsage,
};

fn record(kind: TrajectoryRecordKind) -> TrajectoryRecord {
    TrajectoryRecord {
        id: "step:3:assistant".to_string(),
        index: 4,
        kind,
        step_index: 3,
        turn_index: Some(2),
        preview: "full preview".to_string(),
        started_ms: None,
        finished_ms: None,
        usage: MetricAvailability::Unavailable,
        metrics: TrajectoryMetrics::default(),
        detail: TrajectoryRecordDetail::default(),
    }
}

#[test]
fn trajectory_details_summarizes_available_metrics() {
    let mut assistant = record(TrajectoryRecordKind::Assistant);
    assistant.started_ms = Some(1_000);
    assistant.finished_ms = Some(2_250);
    assistant.usage = MetricAvailability::Available(TrajectoryUsage::new(12, 7, 3, 2));
    assistant.metrics.duration_ms = MetricAvailability::Available(1_250);
    assistant.detail.reasoning = Some("完整推理".to_string());
    assistant.detail.output = Some("完整回答".to_string());

    let details = build_trajectory_details(&assistant);

    assert_eq!(details.summary_value("Source"), Some("Assistant"));
    assert_eq!(details.summary_value("Status"), Some("Completed"));
    assert_eq!(
        details.summary_value("Tokens"),
        Some("input 12 · output 7 · cached 3 · reasoning 2")
    );
    assert_eq!(details.summary_value("Reasoning"), Some("完整推理"));
    assert_eq!(details.summary_value("Content"), Some("完整回答"));
    assert_eq!(details.summary_value("Started"), Some("1000 ms"));
    assert_eq!(details.summary_value("Total duration"), Some("1.2 s"));
    assert_eq!(details.preview_value("Reasoning"), Some("完整推理"));
    assert_eq!(details.preview_value("Content"), Some("完整回答"));
}

#[test]
fn trajectory_details_preserves_raw_payload() {
    let mut tool = record(TrajectoryRecordKind::Tool);
    tool.id = "tool:call-7".to_string();
    tool.detail.tool_name = Some("shell".to_string());
    tool.detail.call_id = Some("call-7".to_string());
    tool.detail.arguments = Some(r#"{"cmd":"pwd"}"#.to_string());
    tool.detail.result = Some("/workspace".to_string());
    tool.detail.raw_fragments = vec![
        json!({
            "id": "call-7",
            "function": {"name": "shell", "arguments": "{\"cmd\":\"pwd\"}"}
        }),
        json!({"role": "tool", "tool_call_id": "call-7", "content": "/workspace"}),
    ];

    let details = build_trajectory_details(&tool);

    assert_eq!(details.summary_value("Source"), Some("Tool · shell"));
    assert_eq!(details.summary_value("Status"), Some("Completed"));
    assert_eq!(details.preview_value("Arguments"), Some(r#"{"cmd":"pwd"}"#));
    assert_eq!(details.preview_value("Result"), Some("/workspace"));
    let raw: serde_json::Value = serde_json::from_str(&details.raw).expect("pretty raw JSON");
    assert_eq!(raw[0]["function"]["arguments"], json!(r#"{"cmd":"pwd"}"#));
    assert_eq!(raw[1]["content"], json!("/workspace"));

    let mut assistant = record(TrajectoryRecordKind::Assistant);
    assistant.detail.raw_fragments =
        vec![json!({"text": "answer", "reasoning_content": "keep this reasoning"})];
    let assistant_raw: serde_json::Value =
        serde_json::from_str(&build_trajectory_details(&assistant).raw)
            .expect("assistant raw JSON");
    assert_eq!(assistant_raw["reasoning_content"], json!("keep this reasoning"));
}

#[test]
fn trajectory_details_trims_input_and_builds_single_line_tooltip_view() {
    let mut user = record(TrajectoryRecordKind::User);
    user.detail.input = Some(format!("  第一行\n\t{}  ", "内容 ".repeat(60)));

    let details = build_trajectory_details(&user);
    let input = details.preview_value("Input").expect("input preview");
    assert!(input.starts_with("第一行"));
    assert!(input.ends_with("内容"));
    let preview = input_preview_text(input);
    assert!(!preview.contains(['\n', '\t']));
    assert!(preview.ends_with('…'));

    let element = trajectory_details(
        &user,
        TrajectoryDetailsTab::Preview,
        TrajectoryDetailsAction::SelectTab,
        TrajectoryDetailsAction::Close,
    );
    let tree = Tree::new(element.as_widget());
    assert!(!tree.children.is_empty());
}

#[test]
fn trajectory_details_uses_explicit_unknown_values() {
    for (kind, source) in [
        (TrajectoryRecordKind::System, "System"),
        (TrajectoryRecordKind::User, "User"),
        (TrajectoryRecordKind::Assistant, "Assistant"),
        (TrajectoryRecordKind::Tool, "Tool"),
    ] {
        let details = build_trajectory_details(&record(kind));
        assert_eq!(details.summary_value("Source"), Some(source));
        assert_eq!(details.summary_value("Status"), Some("—"));
        assert_eq!(details.summary_value("Tokens"), Some("—"));
        assert_eq!(details.summary_value("Reasoning"), Some("—"));
        assert_eq!(details.summary_value("Content"), Some("—"));
        assert_eq!(details.summary_value("Started"), Some("—"));
        assert_eq!(details.summary_value("Total duration"), Some("—"));
        assert_eq!(details.raw, "—");
    }
}

#[test]
fn trajectory_details_tab_and_close_actions_are_observable() {
    let mut state = TrajectoryDetailsState::default();
    assert!(state.select("step:3:assistant"));
    assert_eq!(state.selected_record_id(), Some("step:3:assistant"));
    assert_eq!(state.active_tab(), TrajectoryDetailsTab::Summary);

    assert!(state.apply(TrajectoryDetailsAction::SelectTab(TrajectoryDetailsTab::Raw)));
    assert_eq!(state.active_tab(), TrajectoryDetailsTab::Raw);
    assert!(state.apply(TrajectoryDetailsAction::Close));
    assert_eq!(state.selected_record_id(), None);
    assert_eq!(state.active_tab(), TrajectoryDetailsTab::Summary);
    assert!(!state.apply(TrajectoryDetailsAction::Close));

    let assistant = record(TrajectoryRecordKind::Assistant);
    let element = trajectory_details(
        &assistant,
        TrajectoryDetailsTab::Preview,
        TrajectoryDetailsAction::SelectTab,
        TrajectoryDetailsAction::Close,
    );
    let tree = Tree::new(element.as_widget());
    assert!(!tree.children.is_empty());
}

#[test]
fn trajectory_details_wraps_payloads_and_uses_safe_fonts() {
    assert_eq!(DETAILS_SCROLLBAR_WIDTH, 4.0);
    assert_eq!(
        details_scroll_direction(),
        Direction::Vertical(
            Scrollbar::new().width(DETAILS_SCROLLBAR_WIDTH).scroller_width(DETAILS_SCROLLBAR_WIDTH)
        )
    );
    assert_eq!(detail_payload_font(r#"{"path":"/tmp/a.rs"}"#), Font::MONOSPACE);
    assert_eq!(detail_payload_font("中文内容"), Font::DEFAULT);
    assert_eq!(detail_payload_wrapping(), iced::widget::text::Wrapping::WordOrGlyph);

    let mut assistant = record(TrajectoryRecordKind::Assistant);
    assistant.detail.output = Some("x".repeat(20_000));
    let element = trajectory_details(
        &assistant,
        TrajectoryDetailsTab::Preview,
        TrajectoryDetailsAction::SelectTab,
        TrajectoryDetailsAction::Close,
    );
    let tree = Tree::new(element.as_widget());
    assert!(!tree.children.is_empty());
}

#[test]
fn trajectory_controls_use_rounded_search_and_close_styles() {
    for theme in [iced::Theme::Dark, iced::Theme::Light] {
        let search =
            super::styles::trajectory_search_input_style(&theme, text_input::Status::Active);
        assert_eq!(search.border.radius, 12.0.into());

        for status in [button::Status::Active, button::Status::Hovered, button::Status::Pressed] {
            let close = super::styles::details_close_button_style(&theme, status);
            assert_eq!(close.border.radius, 999.0.into());
        }

        let panel = super::styles::details_panel_style(&theme);
        assert_eq!(panel.shadow.blur_radius, 18.0);
    }
}
