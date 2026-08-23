//! 验证轨迹账本的轮次分组、筛选、折叠控制与多语言渲染。
//! 搜索可临时揭示折叠组内命中记录，关闭搜索后仍由原折叠状态控制可见性。

use iced::advanced::widget::Tree;
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{button, container};
use iced::{Font, Theme};

use super::ledger::{
    LedgerFoldAction, MAX_LEDGER_SEARCH_RESULTS, build_ledger_groups, filter_ledger_records,
    ledger_group_header, ledger_tool_group_button, trajectory_ledger,
};
use super::styles::{
    LEDGER_SCROLLBAR_WIDTH, ledger_record_row_style, ledger_scroll_direction, record_preview_font,
    role_badge_style,
};
use super::{
    MetricAvailability, TrajectoryControlsState, TrajectoryMetrics, TrajectoryProjection,
    TrajectoryRecord, TrajectoryRecordDetail, TrajectoryRecordKind,
};

#[derive(Debug, Clone, PartialEq, Eq)]
enum TestMessage {
    Select(String),
    Fold(LedgerFoldAction),
}

fn record(
    id: &str,
    index: usize,
    kind: TrajectoryRecordKind,
    step_index: u32,
    turn_index: Option<u32>,
    preview: &str,
) -> TrajectoryRecord {
    TrajectoryRecord {
        id: id.to_string(),
        index,
        kind,
        step_index,
        turn_index,
        preview: preview.to_string(),
        started_ms: None,
        finished_ms: None,
        usage: MetricAvailability::Unavailable,
        metrics: TrajectoryMetrics::default(),
        detail: TrajectoryRecordDetail::default(),
    }
}

fn projection(records: Vec<TrajectoryRecord>) -> TrajectoryProjection {
    TrajectoryProjection { records, ..TrajectoryProjection::default() }
}

// 账本 fixture 同时包含用户、助手、推理和工具记录，以验证轮次内的稳定分组顺序。
#[test]
fn trajectory_ledger_groups_turns_and_steps() {
    let projection = projection(vec![
        record("turn:2:assistant", 5, TrajectoryRecordKind::Assistant, 3, Some(2), "later"),
        record("tool:call-1", 4, TrajectoryRecordKind::Tool, 2, Some(1), "sunny"),
        record("turn:1:user", 2, TrajectoryRecordKind::User, 1, Some(1), "weather?"),
        record("step:1:assistant", 3, TrajectoryRecordKind::Assistant, 1, Some(1), "checking"),
        record("system:0", 1, TrajectoryRecordKind::System, 1, None, "instructions"),
        record("context:0", 6, TrajectoryRecordKind::Context, 1, None, "skill catalog"),
    ]);

    let controls = TrajectoryControlsState::default();
    let groups = build_ledger_groups(&projection, "", Some("tool:call-1"), &controls);

    assert_eq!(
        groups.iter().map(|group| group.title.as_str()).collect::<Vec<_>>(),
        ["上下文", "轮次 1", "轮次 2"]
    );
    assert_eq!(
        groups
            .iter()
            .flat_map(|group| group.steps.iter())
            .map(|step| step.title.as_str())
            .collect::<Vec<_>>(),
        ["步骤 1", "步骤 1", "步骤 2", "步骤 3"]
    );
    assert_eq!(
        groups
            .iter()
            .flat_map(|group| group.steps.iter())
            .flat_map(|step| step.records.iter())
            .map(|row| row.record.id.as_str())
            .collect::<Vec<_>>(),
        [
            "system:0",
            "context:0",
            "turn:1:user",
            "step:1:assistant",
            "tool:call-1",
            "turn:2:assistant",
        ]
    );
    let selected = groups
        .iter()
        .flat_map(|group| group.steps.iter())
        .flat_map(|step| step.records.iter())
        .filter(|row| row.selected)
        .map(|row| row.record.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(selected, ["tool:call-1"]);

    let element = trajectory_ledger(
        &projection,
        "",
        Some("tool:call-1"),
        &controls,
        TestMessage::Select,
        TestMessage::Fold,
    );
    let tree = Tree::new(element.as_widget());
    assert_eq!(tree.children.len(), 1);
}

#[test]
fn trajectory_ledger_filter_matches_visible_fields() {
    let mut tool = record("tool:read", 3, TrajectoryRecordKind::Tool, 2, Some(1), "读取上海天气");
    tool.detail.tool_name = Some("Read_File".to_string());
    tool.detail.arguments = Some("hidden-only-argument".to_string());
    let filtered_projection = projection(vec![
        record("user:1", 1, TrajectoryRecordKind::User, 1, Some(1), "请查询"),
        record("assistant:1", 2, TrajectoryRecordKind::Assistant, 1, Some(1), "正在处理"),
        tool,
    ]);

    assert_eq!(ids(filter_ledger_records(&filtered_projection, "AsSiStAnT")), ["assistant:1"]);
    assert_eq!(ids(filter_ledger_records(&filtered_projection, "read_FILE")), ["tool:read"]);
    assert_eq!(ids(filter_ledger_records(&filtered_projection, "上海")), ["tool:read"]);
    assert!(filter_ledger_records(&filtered_projection, "hidden-only-argument").is_empty());

    let many = projection(
        (0..=MAX_LEDGER_SEARCH_RESULTS)
            .map(|index| {
                record(
                    &format!("assistant:{index}"),
                    index,
                    TrajectoryRecordKind::Assistant,
                    index as u32,
                    Some(1),
                    "match",
                )
            })
            .collect(),
    );
    assert_eq!(filter_ledger_records(&many, "match").len(), MAX_LEDGER_SEARCH_RESULTS);
}

#[test]
fn trajectory_ledger_preserves_non_ascii_default_font() {
    assert_eq!(record_preview_font("中文摘要"), Font::DEFAULT);
    assert_eq!(LEDGER_SCROLLBAR_WIDTH, 4.0);
    assert_eq!(
        ledger_scroll_direction(),
        Direction::Vertical(
            Scrollbar::new().width(LEDGER_SCROLLBAR_WIDTH).scroller_width(LEDGER_SCROLLBAR_WIDTH)
        )
    );

    for theme in [Theme::Light, Theme::Dark] {
        for kind in [
            TrajectoryRecordKind::System,
            TrajectoryRecordKind::User,
            TrajectoryRecordKind::Context,
            TrajectoryRecordKind::Assistant,
            TrajectoryRecordKind::Tool,
        ] {
            let badge = role_badge_style(&theme, kind);
            assert!(badge.background.is_some());
            assert_ne!(badge.text_color, Some(theme.palette().background));
        }

        let normal = ledger_record_row_style(&theme, button::Status::Active, false, false);
        let selected = ledger_record_row_style(&theme, button::Status::Active, true, false);
        let missing = ledger_record_row_style(&theme, button::Status::Active, false, true);
        assert_ne!(normal.background, selected.background);
        assert_ne!(normal.text_color, missing.text_color);
        assert_eq!(selected.border.width, 1.0);
    }

    let _container_style: container::Style =
        role_badge_style(&Theme::Dark, TrajectoryRecordKind::Assistant);
}

fn ids(records: Vec<&TrajectoryRecord>) -> Vec<&str> {
    records.into_iter().map(|record| record.id.as_str()).collect()
}

#[test]
fn ledger_headers_emit_fold_messages() {
    use std::cell::RefCell;
    use std::rc::Rc;

    let actions = Rc::new(RefCell::new(Vec::new()));
    let turn_actions = Rc::clone(&actions);
    let turn_header = ledger_group_header("轮次 4", Some(4), false, move |action| {
        turn_actions.borrow_mut().push(action);
        TestMessage::Fold(action)
    });
    let turn_tree = Tree::new(turn_header.as_widget());
    assert!(!turn_tree.children.is_empty());

    let tool_actions = Rc::clone(&actions);
    let tool_button = ledger_tool_group_button(9, true, move |action| {
        tool_actions.borrow_mut().push(action);
        TestMessage::Fold(action)
    });
    let tool_tree = Tree::new(tool_button.as_widget());
    assert!(!tool_tree.children.is_empty());

    assert_eq!(
        actions.borrow().as_slice(),
        [LedgerFoldAction::Turn(4), LedgerFoldAction::ToolStep(9)]
    );
}

mod trajectory_controls {
    use super::*;
    use crate::app::components::chat_panel::trajectory::TrajectoryTurn;

    fn collapsible_projection() -> TrajectoryProjection {
        TrajectoryProjection {
            records: vec![
                record("user:1", 1, TrajectoryRecordKind::User, 1, Some(1), "question"),
                record("assistant:1", 2, TrajectoryRecordKind::Assistant, 1, Some(1), "working"),
                record("tool:1", 3, TrajectoryRecordKind::Tool, 1, Some(1), "needle"),
                record("user:2", 4, TrajectoryRecordKind::User, 2, Some(2), "next"),
            ],
            turns: vec![
                TrajectoryTurn {
                    index: 1,
                    user_record_id: Some("user:1".to_string()),
                    step_indices: vec![1],
                    record_ids: vec![
                        "user:1".to_string(),
                        "assistant:1".to_string(),
                        "tool:1".to_string(),
                    ],
                },
                TrajectoryTurn {
                    index: 2,
                    user_record_id: Some("user:2".to_string()),
                    step_indices: vec![2],
                    record_ids: vec!["user:2".to_string()],
                },
            ],
            ..TrajectoryProjection::default()
        }
    }

    #[test]
    fn turns_toggle_collapses_all_groups() {
        let projection = collapsible_projection();
        let mut controls = TrajectoryControlsState::default();

        controls.toggle_all_turns(&projection);
        let collapsed = build_ledger_groups(&projection, "", None, &controls);
        assert_eq!(collapsed.len(), 2);
        assert!(collapsed.iter().all(|group| group.collapsed));
        assert!(collapsed.iter().all(|group| group.steps.is_empty()));

        controls.toggle_turn(1);
        let partly_expanded = build_ledger_groups(&projection, "", None, &controls);
        assert!(!partly_expanded[0].collapsed);
        assert!(partly_expanded[1].collapsed);

        controls.toggle_all_turns(&projection);
        let collapsed_again = build_ledger_groups(&projection, "", None, &controls);
        assert!(collapsed_again.iter().all(|group| group.collapsed));

        controls.toggle_all_turns(&projection);
        let expanded = build_ledger_groups(&projection, "", None, &controls);
        assert!(expanded.iter().all(|group| !group.collapsed));
    }

    #[test]
    fn calls_toggle_hides_tool_rows() {
        let projection = collapsible_projection();
        let mut controls = TrajectoryControlsState::default();

        controls.toggle_all_calls(&projection);
        let groups = build_ledger_groups(&projection, "", None, &controls);
        let visible_ids = groups
            .iter()
            .flat_map(|group| &group.steps)
            .flat_map(|step| &step.records)
            .map(|row| row.record.id.as_str())
            .collect::<Vec<_>>();
        assert!(visible_ids.contains(&"assistant:1"));
        assert!(!visible_ids.contains(&"tool:1"));

        controls.toggle_tool_step(1);
        let expanded = build_ledger_groups(&projection, "", None, &controls);
        assert!(
            expanded
                .iter()
                .flat_map(|group| &group.steps)
                .flat_map(|step| &step.records)
                .any(|row| row.record.id == "tool:1")
        );
    }

    #[test]
    fn search_reveals_matching_collapsed_record() {
        let projection = collapsible_projection();
        let mut controls = TrajectoryControlsState::default();
        controls.toggle_all_turns(&projection);
        controls.toggle_all_calls(&projection);

        let groups = build_ledger_groups(&projection, "needle", None, &controls);
        assert_eq!(groups.len(), 1);
        assert!(!groups[0].collapsed);
        assert_eq!(groups[0].steps[0].records[0].record.id, "tool:1");
        assert!(controls.is_turn_collapsed(1));
        assert!(controls.is_tool_step_collapsed(1));
    }
}
