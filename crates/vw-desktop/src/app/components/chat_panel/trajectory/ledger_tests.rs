use iced::advanced::widget::Tree;
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{button, container};
use iced::{Font, Theme};

use super::ledger::{
    MAX_LEDGER_SEARCH_RESULTS, build_ledger_groups, filter_ledger_records, trajectory_ledger,
};
use super::styles::{
    LEDGER_SCROLLBAR_WIDTH, ledger_record_row_style, ledger_scroll_direction, record_preview_font,
    role_badge_style,
};
use super::{
    MetricAvailability, TrajectoryMetrics, TrajectoryProjection, TrajectoryRecord,
    TrajectoryRecordDetail, TrajectoryRecordKind,
};

#[derive(Debug, Clone, PartialEq, Eq)]
enum TestMessage {
    Select(String),
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

#[test]
fn trajectory_ledger_groups_turns_and_steps() {
    let projection = projection(vec![
        record("turn:2:assistant", 5, TrajectoryRecordKind::Assistant, 3, Some(2), "later"),
        record("tool:call-1", 4, TrajectoryRecordKind::Tool, 2, Some(1), "sunny"),
        record("turn:1:user", 2, TrajectoryRecordKind::User, 1, Some(1), "weather?"),
        record("step:1:assistant", 3, TrajectoryRecordKind::Assistant, 1, Some(1), "checking"),
        record("system:0", 1, TrajectoryRecordKind::System, 1, None, "instructions"),
    ]);

    let groups = build_ledger_groups(&projection, "", Some("tool:call-1"));

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
        ["system:0", "turn:1:user", "step:1:assistant", "tool:call-1", "turn:2:assistant"]
    );
    let selected = groups
        .iter()
        .flat_map(|group| group.steps.iter())
        .flat_map(|step| step.records.iter())
        .filter(|row| row.selected)
        .map(|row| row.record.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(selected, ["tool:call-1"]);

    let element = trajectory_ledger(&projection, "", Some("tool:call-1"), TestMessage::Select);
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
