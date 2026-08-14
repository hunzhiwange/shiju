use std::cell::RefCell;
use std::rc::Rc;

use iced::Theme;
use iced::advanced::widget::Tree;
use iced::widget::button;

use super::styles::timeline_block_style;
use super::timeline::{
    MIN_TIMELINE_BLOCK_UNITS, TimelineLane, build_timeline, format_timeline_duration,
    trajectory_timeline,
};
use super::{
    MetricAvailability, TrajectoryMetrics, TrajectoryProjection, TrajectoryRecord,
    TrajectoryRecordDetail, TrajectoryRecordKind, TrajectoryTurn,
};

#[derive(Debug, Clone, PartialEq, Eq)]
enum TestMessage {
    Select(String),
}

fn record(id: &str, index: usize, kind: TrajectoryRecordKind) -> TrajectoryRecord {
    TrajectoryRecord {
        id: id.to_string(),
        index,
        kind,
        step_index: index as u32,
        turn_index: (kind != TrajectoryRecordKind::System).then_some(1),
        preview: id.to_string(),
        started_ms: None,
        finished_ms: None,
        usage: MetricAvailability::Unavailable,
        metrics: TrajectoryMetrics::default(),
        detail: TrajectoryRecordDetail::default(),
    }
}

fn turn(index: u32) -> TrajectoryTurn {
    TrajectoryTurn { index, user_record_id: None, step_indices: Vec::new(), record_ids: Vec::new() }
}

#[test]
fn trajectory_timeline_assigns_three_lanes() {
    let projection = TrajectoryProjection {
        records: vec![
            record("system:stable", 1, TrajectoryRecordKind::System),
            record("user:stable", 2, TrajectoryRecordKind::User),
            record("assistant:stable", 3, TrajectoryRecordKind::Assistant),
            record("tool:stable", 4, TrajectoryRecordKind::Tool),
        ],
        ..TrajectoryProjection::default()
    };

    let timeline = build_timeline(&projection);

    assert_eq!(
        timeline.blocks.iter().map(|block| block.lane).collect::<Vec<_>>(),
        [TimelineLane::Input, TimelineLane::Input, TimelineLane::Model, TimelineLane::Tools]
    );
    assert!(timeline.blocks.iter().all(|block| block.normalized_width > 0.0));
    assert_eq!(timeline.blocks[0].units, MIN_TIMELINE_BLOCK_UNITS);
    assert_eq!(timeline.blocks[2].units, MIN_TIMELINE_BLOCK_UNITS);
    assert_eq!(timeline.blocks[3].units, MIN_TIMELINE_BLOCK_UNITS);
    let total_width = timeline.blocks.iter().map(|block| block.normalized_width).sum::<f32>();
    assert!((total_width - 1.0).abs() < f32::EPSILON * 8.0);
}

#[test]
fn trajectory_timeline_counts_only_recorded_metrics() {
    let mut input = record("user:1", 1, TrajectoryRecordKind::User);
    input.metrics.duration_ms = MetricAvailability::Available(8_000);
    let mut recorded_model = record("assistant:1", 2, TrajectoryRecordKind::Assistant);
    recorded_model.metrics.duration_ms = MetricAvailability::Available(450);
    let unknown_model = record("assistant:2", 3, TrajectoryRecordKind::Assistant);
    let mut tool = record("tool:call-1", 4, TrajectoryRecordKind::Tool);
    tool.metrics.duration_ms = MetricAvailability::Available(12_000);
    tool.metrics.tool_duration_ms = MetricAvailability::Available(12_000);
    let projection = TrajectoryProjection {
        records: vec![input, recorded_model, unknown_model, tool],
        turns: vec![turn(1), turn(2)],
        ..TrajectoryProjection::default()
    };

    let timeline = build_timeline(&projection);

    assert_eq!(timeline.summary.duration_ms, MetricAvailability::Available(450));
    assert_eq!(timeline.summary.turns, 2);
    assert_eq!(timeline.summary.calls, 1);
    assert_eq!(timeline.blocks[1].recorded_duration_ms, Some(450));
    assert_eq!(timeline.blocks[2].recorded_duration_ms, None);
    assert_eq!(timeline.blocks[3].recorded_duration_ms, None);
    assert!(timeline.blocks[2].normalized_width > 0.0);

    let unknown = TrajectoryProjection {
        records: vec![record("assistant:unknown", 1, TrajectoryRecordKind::Assistant)],
        ..TrajectoryProjection::default()
    };
    let unknown_summary = build_timeline(&unknown).summary;
    assert_eq!(unknown_summary.duration_ms, MetricAvailability::Unavailable);
    assert_eq!(format_timeline_duration(unknown_summary.duration_ms), "—");
}

#[test]
fn trajectory_timeline_selects_stable_record() {
    let projection = TrajectoryProjection {
        records: vec![
            record("step:1:message:0:user", 1, TrajectoryRecordKind::User),
            record("step:1:assistant", 2, TrajectoryRecordKind::Assistant),
            record("tool:stable-call", 3, TrajectoryRecordKind::Tool),
        ],
        ..TrajectoryProjection::default()
    };
    let selected_ids = Rc::new(RefCell::new(Vec::new()));
    let captured_ids = Rc::clone(&selected_ids);

    let element = trajectory_timeline(&projection, Some("tool:stable-call"), move |record_id| {
        captured_ids.borrow_mut().push(record_id.clone());
        TestMessage::Select(record_id)
    });
    let tree = Tree::new(element.as_widget());

    assert!(!tree.children.is_empty());
    assert_eq!(
        selected_ids.borrow().as_slice(),
        ["step:1:message:0:user", "step:1:assistant", "tool:stable-call"]
    );

    for theme in [Theme::Light, Theme::Dark] {
        for lane in [TimelineLane::Input, TimelineLane::Model, TimelineLane::Tools] {
            let normal = timeline_block_style(&theme, button::Status::Active, lane, false);
            let selected = timeline_block_style(&theme, button::Status::Active, lane, true);
            assert!(normal.background.is_some());
            assert_ne!(normal.background, selected.background);
            assert_eq!(selected.border.width, 1.0);
        }
    }
}
