//! Input / Model / Tools 三泳道轨迹概览。

use iced::widget::text::Wrapping;
use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Element, Length, Theme};

use super::styles;
use super::{
    MetricAvailability, TrajectoryDurationMode, TrajectoryProjection, TrajectoryRecord,
    TrajectoryRecordKind,
};
use crate::app::components::chat_panel::utils::truncate_chars;

/// 无可信时长的记录仍保留一个可见、可点击的布局单元。
pub(crate) const MIN_TIMELINE_BLOCK_UNITS: u64 = 120;
const TIMELINE_FILL_PORTIONS: f32 = 1_000.0;
const TIMELINE_TOOLTIP_PREVIEW_CHAR_LIMIT: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum TimelineLane {
    Input,
    Model,
    Tools,
}

impl TimelineLane {
    const ALL: [Self; 3] = [Self::Input, Self::Model, Self::Tools];

    const fn label(self) -> &'static str {
        match self {
            Self::Input => "Input",
            Self::Model => "Model",
            Self::Tools => "Tools",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TimelineSummary {
    pub(crate) duration_ms: MetricAvailability<u64>,
    pub(crate) turns: usize,
    pub(crate) calls: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TimelineToolbarAction {
    ToggleDuration,
    ToggleTurns,
    ToggleCalls,
}

#[derive(Debug)]
pub(crate) struct TimelineBlock<'a> {
    pub(crate) record: &'a TrajectoryRecord,
    pub(crate) lane: TimelineLane,
    pub(crate) recorded_duration_ms: Option<u64>,
    pub(crate) units: u64,
    pub(crate) normalized_start: f32,
    pub(crate) normalized_width: f32,
}

#[derive(Debug)]
pub(crate) struct TrajectoryTimeline<'a> {
    pub(crate) blocks: Vec<TimelineBlock<'a>>,
    pub(crate) summary: TimelineSummary,
}

/// 将记录转换为与窗口像素宽度无关的归一化布局。
pub(crate) fn build_timeline(
    projection: &TrajectoryProjection,
    duration_mode: TrajectoryDurationMode,
) -> TrajectoryTimeline<'_> {
    let mut records = projection.records.iter().collect::<Vec<_>>();
    records.sort_by_key(|record| record.index);

    let total_units = records
        .iter()
        .map(|record| block_units(record, duration_mode))
        .fold(0_u64, u64::saturating_add);
    let mut elapsed_units = 0_u64;
    let mut blocks = Vec::with_capacity(records.len());
    for record in records {
        let recorded_duration_ms = recorded_timeline_duration(record);
        let units = block_units(record, duration_mode);
        let normalized_start = normalize_units(elapsed_units, total_units);
        let normalized_width = normalize_units(units, total_units);
        blocks.push(TimelineBlock {
            record,
            lane: lane_for(record.kind),
            recorded_duration_ms,
            units,
            normalized_start,
            normalized_width,
        });
        elapsed_units = elapsed_units.saturating_add(units);
    }

    TrajectoryTimeline { blocks, summary: build_timeline_summary(projection) }
}

pub(crate) fn trajectory_timeline<'a, Message: Clone + 'a>(
    projection: &TrajectoryProjection,
    duration_mode: TrajectoryDurationMode,
    turns_collapsed: bool,
    calls_collapsed: bool,
    selected_record_id: Option<&str>,
    on_control: impl Fn(TimelineToolbarAction) -> Message,
    on_select: impl Fn(String) -> Message,
) -> Element<'a, Message> {
    let timeline = build_timeline(projection, duration_mode);
    let toolbar = timeline_toolbar(
        timeline.summary,
        duration_mode,
        turns_collapsed,
        calls_collapsed,
        on_control,
    );
    let mut lanes = column![].spacing(6).width(Length::Fill);

    for lane in TimelineLane::ALL {
        let mut track = row![].spacing(2).width(Length::Fill).align_y(Alignment::Center);
        for block in &timeline.blocks {
            let portion = fill_portion(block.normalized_width);
            if block.lane == lane {
                let selected = selected_record_id.is_some_and(|id| id == block.record.id);
                let record_kind = block.record.kind;
                let tooltip_text = timeline_block_tooltip(block.record);
                let block_button = button(text(""))
                    .padding(0)
                    .width(Length::Fill)
                    .height(Length::Fixed(18.0))
                    .style(move |theme, status| {
                        styles::timeline_block_style(theme, status, record_kind, selected)
                    })
                    .on_press(on_select(block.record.id.clone()));
                track = track.push(
                    container(
                        Tooltip::new(
                            block_button,
                            container(text(tooltip_text).size(11).wrapping(Wrapping::None))
                                .padding([6, 8])
                                .style(styles::timeline_tooltip_style),
                            TooltipPosition::FollowCursor,
                        )
                        .gap(6),
                    )
                    .width(Length::FillPortion(portion))
                    .height(Length::Fixed(18.0)),
                );
            } else {
                track = track.push(
                    container(text(""))
                        .width(Length::FillPortion(portion))
                        .height(Length::Fixed(18.0)),
                );
            }
        }
        lanes = lanes.push(
            row![
                container(text(lane.label()).size(11).style(move |theme| {
                    iced::widget::text::Style {
                        color: Some(styles::timeline_lane_text_color(theme, lane)),
                    }
                }))
                .width(Length::Fixed(48.0)),
                container(track).width(Length::Fill).style(styles::timeline_track_style),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        );
    }

    container(column![toolbar, lanes].spacing(8))
        .padding(10)
        .width(Length::Fill)
        .style(styles::timeline_panel_style)
        .into()
}

pub(crate) fn timeline_toolbar<'a, Message: Clone + 'a>(
    summary: TimelineSummary,
    duration_mode: TrajectoryDurationMode,
    turns_collapsed: bool,
    calls_collapsed: bool,
    on_control: impl Fn(TimelineToolbarAction) -> Message,
) -> Element<'a, Message> {
    let duration_state = match duration_mode {
        TrajectoryDurationMode::Timed => "真实",
        TrajectoryDurationMode::Equal => "等宽",
    };
    row![
        toolbar_button(
            "Duration",
            format!("{duration_state} · {}", format_timeline_duration(summary.duration_ms)),
            duration_mode == TrajectoryDurationMode::Timed,
            on_control(TimelineToolbarAction::ToggleDuration),
        ),
        toolbar_button(
            "Turns",
            format!("{} · {}", fold_arrow(turns_collapsed), summary.turns),
            turns_collapsed,
            on_control(TimelineToolbarAction::ToggleTurns),
        ),
        toolbar_button(
            "Calls",
            format!("{} · {}", fold_arrow(calls_collapsed), summary.calls),
            calls_collapsed,
            on_control(TimelineToolbarAction::ToggleCalls),
        ),
    ]
    .spacing(6)
    .width(Length::Fill)
    .into()
}

pub(crate) fn timeline_block_tooltip(record: &TrajectoryRecord) -> String {
    let source = match record.kind {
        TrajectoryRecordKind::System => "SYSTEM".to_string(),
        TrajectoryRecordKind::User => "USER".to_string(),
        TrajectoryRecordKind::Context => "CONTEXT".to_string(),
        TrajectoryRecordKind::Assistant => "ASSISTANT".to_string(),
        TrajectoryRecordKind::Tool => record
            .detail
            .tool_name
            .as_deref()
            .map_or_else(|| "TOOL".to_string(), |name| format!("TOOL ({})", clean_line(name))),
    };
    let preview = truncate_chars(&full_preview(record), TIMELINE_TOOLTIP_PREVIEW_CHAR_LIMIT);
    let preview = if preview.is_empty() { "—" } else { preview.as_str() };
    let turn = record.turn_index.map_or_else(|| "不可用".to_string(), |turn| turn.to_string());
    let duration = recorded_timeline_duration(record).map_or_else(
        || "不可用".to_string(),
        |duration| format_timeline_duration(MetricAvailability::Available(duration)),
    );
    format!("{source} · {preview} · step {} · turn {turn} · duration {duration}", record.step_index)
}

pub(crate) fn format_timeline_duration(duration_ms: MetricAvailability<u64>) -> String {
    match duration_ms {
        MetricAvailability::Available(duration_ms) if duration_ms < 1_000 => {
            format!("{duration_ms} ms")
        }
        MetricAvailability::Available(duration_ms) => {
            format!("{}.{:01} s", duration_ms / 1_000, duration_ms % 1_000 / 100)
        }
        MetricAvailability::Unavailable => "—".to_string(),
    }
}

fn toolbar_button<'a, Message: Clone + 'a>(
    label: &'static str,
    value: String,
    active: bool,
    message: Message,
) -> Element<'a, Message> {
    button(
        row![text(label).size(10), text(value).size(11).wrapping(Wrapping::None),]
            .spacing(5)
            .align_y(Alignment::Center),
    )
    .padding([5, 7])
    .width(Length::FillPortion(1))
    .style(move |theme: &Theme, status| {
        styles::timeline_toolbar_button_style(theme, status, active)
    })
    .on_press(message)
    .into()
}

const fn fold_arrow(collapsed: bool) -> &'static str {
    if collapsed { "▸" } else { "▾" }
}

fn clean_line(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn full_preview(record: &TrajectoryRecord) -> String {
    let full = match record.kind {
        TrajectoryRecordKind::System
        | TrajectoryRecordKind::User
        | TrajectoryRecordKind::Context => record.detail.input.as_deref(),
        TrajectoryRecordKind::Assistant => record
            .detail
            .output
            .as_deref()
            .filter(|output| !output.trim().is_empty())
            .or(record.detail.reasoning.as_deref()),
        TrajectoryRecordKind::Tool => record.detail.arguments.as_deref(),
    };
    full.map_or_else(|| clean_line(&record.preview), clean_line)
}

fn build_timeline_summary(projection: &TrajectoryProjection) -> TimelineSummary {
    let mut duration_ms = 0_u64;
    let mut has_recorded_duration = false;
    let mut calls = 0;
    for record in &projection.records {
        if record.kind == TrajectoryRecordKind::Tool {
            calls += 1;
        }
        if let Some(recorded_duration_ms) = recorded_assistant_duration(record) {
            duration_ms = duration_ms.saturating_add(recorded_duration_ms);
            has_recorded_duration = true;
        }
    }
    TimelineSummary {
        duration_ms: if has_recorded_duration {
            MetricAvailability::Available(duration_ms)
        } else {
            MetricAvailability::Unavailable
        },
        turns: projection.turns.len(),
        calls,
    }
}

const fn lane_for(kind: TrajectoryRecordKind) -> TimelineLane {
    match kind {
        TrajectoryRecordKind::System
        | TrajectoryRecordKind::User
        | TrajectoryRecordKind::Context => TimelineLane::Input,
        TrajectoryRecordKind::Assistant => TimelineLane::Model,
        TrajectoryRecordKind::Tool => TimelineLane::Tools,
    }
}

fn recorded_timeline_duration(record: &TrajectoryRecord) -> Option<u64> {
    match record.kind {
        TrajectoryRecordKind::Assistant => match record.metrics.duration_ms {
            MetricAvailability::Available(duration_ms) => Some(duration_ms),
            MetricAvailability::Unavailable => None,
        },
        TrajectoryRecordKind::Tool => match record.metrics.tool_duration_ms {
            MetricAvailability::Available(duration_ms) => Some(duration_ms),
            MetricAvailability::Unavailable => None,
        },
        TrajectoryRecordKind::System
        | TrajectoryRecordKind::User
        | TrajectoryRecordKind::Context => None,
    }
}

fn recorded_assistant_duration(record: &TrajectoryRecord) -> Option<u64> {
    if record.kind != TrajectoryRecordKind::Assistant {
        return None;
    }
    match record.metrics.duration_ms {
        MetricAvailability::Available(duration_ms) => Some(duration_ms),
        MetricAvailability::Unavailable => None,
    }
}

fn block_units(record: &TrajectoryRecord, duration_mode: TrajectoryDurationMode) -> u64 {
    match duration_mode {
        TrajectoryDurationMode::Timed => recorded_timeline_duration(record)
            .unwrap_or(MIN_TIMELINE_BLOCK_UNITS)
            .max(MIN_TIMELINE_BLOCK_UNITS),
        TrajectoryDurationMode::Equal => MIN_TIMELINE_BLOCK_UNITS,
    }
}

fn normalize_units(units: u64, total_units: u64) -> f32 {
    if total_units == 0 { 0.0 } else { units as f32 / total_units as f32 }
}

fn fill_portion(normalized_width: f32) -> u16 {
    (normalized_width * TIMELINE_FILL_PORTIONS).round().clamp(1.0, u16::MAX as f32) as u16
}
