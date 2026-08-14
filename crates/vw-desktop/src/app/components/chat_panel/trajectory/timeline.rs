//! Input / Model / Tools 三泳道轨迹概览。

use iced::widget::text::Wrapping;
use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Element, Length};

use super::styles;
use super::{MetricAvailability, TrajectoryProjection, TrajectoryRecord, TrajectoryRecordKind};

/// 无可信时长的记录仍保留一个可见、可点击的布局单元。
pub(crate) const MIN_TIMELINE_BLOCK_UNITS: u64 = 120;
const TIMELINE_FILL_PORTIONS: f32 = 1_000.0;

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
pub(crate) fn build_timeline(projection: &TrajectoryProjection) -> TrajectoryTimeline<'_> {
    let mut records = projection.records.iter().collect::<Vec<_>>();
    records.sort_by_key(|record| record.index);

    let total_units =
        records.iter().map(|record| block_units(record)).fold(0_u64, u64::saturating_add);
    let mut elapsed_units = 0_u64;
    let mut blocks = Vec::with_capacity(records.len());
    for record in records {
        let recorded_duration_ms = recorded_model_duration(record);
        let units =
            recorded_duration_ms.unwrap_or(MIN_TIMELINE_BLOCK_UNITS).max(MIN_TIMELINE_BLOCK_UNITS);
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
    selected_record_id: Option<&str>,
    on_select: impl Fn(String) -> Message,
) -> Element<'a, Message> {
    let timeline = build_timeline(projection);
    let summary = row![
        summary_item("Duration", format_timeline_duration(timeline.summary.duration_ms)),
        summary_item("Turns", timeline.summary.turns.to_string()),
        summary_item("Calls", timeline.summary.calls.to_string()),
    ]
    .spacing(8)
    .width(Length::Fill);
    let mut lanes = column![].spacing(6).width(Length::Fill);

    for lane in TimelineLane::ALL {
        let mut track = row![].spacing(2).width(Length::Fill).align_y(Alignment::Center);
        for block in &timeline.blocks {
            let portion = fill_portion(block.normalized_width);
            if block.lane == lane {
                let selected = selected_record_id.is_some_and(|id| id == block.record.id);
                track = track.push(
                    button(text(block.record.preview.clone()).size(10).wrapping(Wrapping::None))
                        .padding([5, 7])
                        .width(Length::FillPortion(portion))
                        .height(Length::Fixed(28.0))
                        .style(move |theme, status| {
                            styles::timeline_block_style(theme, status, lane, selected)
                        })
                        .on_press(on_select(block.record.id.clone())),
                );
            } else {
                track = track.push(
                    container(text(""))
                        .width(Length::FillPortion(portion))
                        .height(Length::Fixed(28.0)),
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

    container(column![summary, lanes].spacing(10))
        .padding(10)
        .width(Length::Fill)
        .style(styles::timeline_panel_style)
        .into()
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

fn summary_item<'a, Message: 'a>(label: &'static str, value: String) -> Element<'a, Message> {
    container(
        column![
            text(label).size(10).style(|theme| iced::widget::text::Style {
                color: Some(styles::timeline_stat_label_color(theme)),
            }),
            text(value).size(14).style(|theme| iced::widget::text::Style {
                color: Some(styles::timeline_stat_value_color(theme)),
            }),
        ]
        .spacing(2),
    )
    .padding([5, 8])
    .width(Length::FillPortion(1))
    .style(styles::timeline_stat_style)
    .into()
}

fn build_timeline_summary(projection: &TrajectoryProjection) -> TimelineSummary {
    let mut duration_ms = 0_u64;
    let mut has_recorded_duration = false;
    let mut calls = 0;
    for record in &projection.records {
        if record.kind == TrajectoryRecordKind::Tool {
            calls += 1;
        }
        if let Some(recorded_duration_ms) = recorded_model_duration(record) {
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
        TrajectoryRecordKind::System | TrajectoryRecordKind::User => TimelineLane::Input,
        TrajectoryRecordKind::Assistant => TimelineLane::Model,
        TrajectoryRecordKind::Tool => TimelineLane::Tools,
    }
}

fn recorded_model_duration(record: &TrajectoryRecord) -> Option<u64> {
    if record.kind != TrajectoryRecordKind::Assistant {
        return None;
    }
    match record.metrics.duration_ms {
        MetricAvailability::Available(duration_ms) => Some(duration_ms),
        MetricAvailability::Unavailable => None,
    }
}

fn block_units(record: &TrajectoryRecord) -> u64 {
    recorded_model_duration(record)
        .unwrap_or(MIN_TIMELINE_BLOCK_UNITS)
        .max(MIN_TIMELINE_BLOCK_UNITS)
}

fn normalize_units(units: u64, total_units: u64) -> f32 {
    if total_units == 0 { 0.0 } else { units as f32 / total_units as f32 }
}

fn fill_portion(normalized_width: f32) -> u16 {
    (normalized_width * TIMELINE_FILL_PORTIONS).round().clamp(1.0, u16::MAX as f32) as u16
}
