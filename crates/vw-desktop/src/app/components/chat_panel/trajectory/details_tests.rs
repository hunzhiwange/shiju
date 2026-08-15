use iced::Font;
use iced::advanced::widget::Tree;
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{button, text_input};
use serde_json::json;

use super::details::{
    DETAILS_SCROLLBAR_WIDTH, build_trajectory_details, detail_payload_font,
    detail_payload_wrapping, details_scroll_direction, input_preview_text,
    trajectory_detail_values, trajectory_details,
};
use super::details_content::{TrajectoryDetailContent, build_details_content};
use super::{
    MetricAvailability, TrajectoryDetailsAction, TrajectoryDetailsState, TrajectoryDetailsTab,
    TrajectoryMetrics, TrajectoryRecord, TrajectoryRecordDetail, TrajectoryRecordKind,
    TrajectoryToolDefinition, TrajectoryUsage,
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

fn details_state(record: &TrajectoryRecord, tab: TrajectoryDetailsTab) -> TrajectoryDetailsState {
    let mut state = TrajectoryDetailsState::default();
    state.select(record.id.clone());
    if tab != TrajectoryDetailsTab::Summary {
        state.apply(TrajectoryDetailsAction::SelectTab(tab));
    }
    state.sync_values(trajectory_detail_values(record, tab));
    state
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

    let state = details_state(&user, TrajectoryDetailsTab::Preview);
    let element = trajectory_details(&user, &state);
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
    let state = details_state(&assistant, TrajectoryDetailsTab::Preview);
    let element = trajectory_details(&assistant, &state);
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
    let state = details_state(&assistant, TrajectoryDetailsTab::Preview);
    let element = trajectory_details(&assistant, &state);
    let tree = Tree::new(element.as_widget());
    assert!(!tree.children.is_empty());
}

#[test]
fn trajectory_details_values_are_selectable_and_copyable_from_context_menu() {
    let assistant = record(TrajectoryRecordKind::Assistant);
    let mut state = details_state(&assistant, TrajectoryDetailsTab::Summary);

    assert!(state.editor(0).is_some());
    state.open_context_menu(0, 12.0, 18.0);
    assert_eq!(state.context_menu_position(0), Some((12.0, 18.0)));
    assert_eq!(state.context_menu_copy_text().as_deref(), Some("—"));

    state.close_context_menu();
    assert_eq!(state.context_menu_position(0), None);
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

        let tool = super::styles::tool_definition_style(&theme);
        assert!(tool.background.is_some());
        assert_eq!(tool.border.width, 1.0);
        assert!(
            super::styles::tool_definition_header_style(&theme, button::Status::Hovered)
                .background
                .is_some()
        );
    }
}

#[test]
fn details_render_exact_reference_tabs_by_kind() {
    let expected = [
        (TrajectoryRecordKind::System, "SYSTEM", "Step 3 · Record #4"),
        (TrajectoryRecordKind::User, "USER", "Turn 2 · Message"),
        (TrajectoryRecordKind::Context, "CONTEXT", "Turn 2 · Message"),
        (TrajectoryRecordKind::Assistant, "MODEL", "Turn 2 · Model"),
        (TrajectoryRecordKind::Tool, "TOOL", "Turn 2 · Tool"),
    ];

    for (kind, badge, location) in expected {
        let record = record(kind);
        let first_tab = kind.detail_tabs()[0];
        let model = build_details_content(&record, first_tab);
        assert_eq!(model.badge, badge);
        assert_eq!(model.location, location);
        assert_eq!(model.tabs, kind.detail_tabs());

        let state = details_state(&record, first_tab);
        assert!(!Tree::new(trajectory_details(&record, &state).as_widget()).children.is_empty());
    }

    for theme in [iced::Theme::Dark, iced::Theme::Light] {
        assert!(super::styles::details_tab_underline_style(&theme, true).background.is_some());
        assert!(super::styles::details_tab_underline_style(&theme, false).background.is_none());
    }
}

#[test]
fn details_render_system_prompt_and_tools() {
    let mut system = record(TrajectoryRecordKind::System);
    system.turn_index = None;
    system.detail.input = Some("# Rules\n\n保留中文换行。".to_string());
    system.detail.request_tools = vec![TrajectoryToolDefinition {
        id: "shell".to_string(),
        description: "Run a shell command".to_string(),
        input_schema: json!({"type": "object", "required": ["command"]}),
    }];
    let expected_tools = system.detail.request_tools.clone();

    assert_eq!(
        build_details_content(&system, TrajectoryDetailsTab::SystemPrompt).body,
        TrajectoryDetailContent::Text("# Rules\n\n保留中文换行。".to_string())
    );
    assert_eq!(
        build_details_content(&system, TrajectoryDetailsTab::Tools).body,
        TrajectoryDetailContent::Tools(expected_tools)
    );

    let state = details_state(&system, TrajectoryDetailsTab::Tools);
    assert!(!Tree::new(trajectory_details(&system, &state).as_widget()).children.is_empty());
}

#[test]
fn system_tools_open_first_definition_and_collapse_the_rest() {
    let mut system = record(TrajectoryRecordKind::System);
    system.detail.request_tools = (0..3)
        .map(|index| TrajectoryToolDefinition {
            id: format!("tool_{index}"),
            description: format!("Tool {index}"),
            input_schema: json!({"type": "object"}),
        })
        .collect();
    let mut state = TrajectoryDetailsState::default();

    assert!(state.select_record(&system));
    assert_eq!(
        state.collapsed_paths_for(TrajectoryDetailsTab::Tools),
        std::collections::HashSet::from(["/tools/1".to_string(), "/tools/2".to_string()])
    );
}

#[test]
fn details_render_user_context_source() {
    for (kind, source) in [
        (
            TrajectoryRecordKind::User,
            json!({"kind": "user", "rpcId": "rpc-7", "clientTimeZone": "Asia/Shanghai"}),
        ),
        (
            TrajectoryRecordKind::Context,
            json!({"kind": "context", "form": "skill_catalog"}),
        ),
    ] {
        let mut message = record(kind);
        message.detail.input = Some("请查询".to_string());
        message.detail.source = Some(source.clone());
        assert_eq!(
            build_details_content(&message, TrajectoryDetailsTab::Preview).body,
            TrajectoryDetailContent::Text("请查询".to_string())
        );
        assert_eq!(
            build_details_content(&message, TrajectoryDetailsTab::Source).body,
            TrajectoryDetailContent::Json(source)
        );
    }

    let legacy = record(TrajectoryRecordKind::User);
    assert_eq!(
        build_details_content(&legacy, TrajectoryDetailsTab::Source).body,
        TrajectoryDetailContent::Unavailable("Source not recorded".to_string())
    );
}

#[test]
fn details_render_model_options_usage_timing() {
    let mut model = record(TrajectoryRecordKind::Assistant);
    model.started_ms = Some(1_000);
    model.finished_ms = Some(2_250);
    model.metrics.duration_ms = MetricAvailability::Available(1_250);
    model.usage = MetricAvailability::Available(TrajectoryUsage::new(12, 7, 3, 2));
    model.detail.output = Some("回答".to_string());
    model.detail.request_options = Some(json!({"model": "provider/model", "temperature": 0.2}));

    assert_eq!(
        build_details_content(&model, TrajectoryDetailsTab::Options).body,
        TrajectoryDetailContent::Json(json!({"model": "provider/model", "temperature": 0.2}))
    );
    let usage = build_details_content(&model, TrajectoryDetailsTab::Usage).body;
    assert!(matches!(usage, TrajectoryDetailContent::Fields(ref fields) if fields.contains(&("Input tokens".to_string(), "12".to_string())) && fields.contains(&("Reasoning tokens".to_string(), "2".to_string()))));
    let timing = build_details_content(&model, TrajectoryDetailsTab::Timing).body;
    assert!(matches!(timing, TrajectoryDetailContent::Fields(ref fields) if fields.contains(&("Started".to_string(), "1000 ms".to_string())) && fields.contains(&("Duration".to_string(), "1.2 s".to_string()))));
}

#[test]
fn details_render_tool_payload_result_schema_timing() {
    let mut tool = record(TrajectoryRecordKind::Tool);
    tool.detail.tool_name = Some("shell".to_string());
    tool.detail.call_id = Some("call-9".to_string());
    tool.detail.arguments = Some(r#"{"command":"pwd"}"#.to_string());
    tool.detail.result = Some(r#"{"stdout":"/workspace"}"#.to_string());
    tool.detail.tool_schema = Some(json!({"type": "object", "required": ["command"]}));
    tool.started_ms = Some(2_000);
    tool.finished_ms = Some(2_080);
    tool.metrics.tool_duration_ms = MetricAvailability::Available(80);

    assert_eq!(
        build_details_content(&tool, TrajectoryDetailsTab::Payload).body,
        TrajectoryDetailContent::Json(json!({"command": "pwd"}))
    );
    assert_eq!(
        build_details_content(&tool, TrajectoryDetailsTab::Result).body,
        TrajectoryDetailContent::Json(json!({"stdout": "/workspace"}))
    );
    assert_eq!(
        build_details_content(&tool, TrajectoryDetailsTab::Schema).body,
        TrajectoryDetailContent::Json(json!({"type": "object", "required": ["command"]}))
    );
    assert!(matches!(
        build_details_content(&tool, TrajectoryDetailsTab::Timing).body,
        TrajectoryDetailContent::Fields(ref fields)
            if fields.contains(&("Duration".to_string(), "80 ms".to_string()))
    ));
}

#[test]
fn details_state_rejects_tabs_not_supported_by_selected_record() {
    let mut state = TrajectoryDetailsState::default();
    let mut system = record(TrajectoryRecordKind::System);
    system.id = "system:1".to_string();
    state.select_record(&system);

    assert!(!state.apply(TrajectoryDetailsAction::SelectTab(TrajectoryDetailsTab::Summary)));
    assert_eq!(state.active_tab(), TrajectoryDetailsTab::SystemPrompt);
    assert!(state.apply(TrajectoryDetailsAction::SelectTab(TrajectoryDetailsTab::Tools)));
    assert!(!state.apply(TrajectoryDetailsAction::SelectTab(TrajectoryDetailsTab::Payload)));
    assert_eq!(state.active_tab(), TrajectoryDetailsTab::Tools);
    state.toggle_json_path("/0/input_schema".to_string());
    assert!(state.json_collapsed_paths().contains("/0/input_schema"));

    let user = record(TrajectoryRecordKind::User);
    assert!(state.select_record(&user));
    assert_eq!(state.active_tab(), TrajectoryDetailsTab::Summary);
    assert!(state.json_collapsed_paths().is_empty());
}
