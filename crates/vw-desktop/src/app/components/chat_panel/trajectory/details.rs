//! 选中轨迹记录的 Summary、Preview 与 Raw 检查器。

use iced::widget::scrollable::Direction;
use iced::widget::text::Wrapping;
use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Alignment, Element, Font, Length, Theme};

use super::styles;
use super::{
    MetricAvailability, TrajectoryDetailsTab, TrajectoryRecord, TrajectoryRecordKind,
    TrajectoryUsage,
};
use crate::app::assets::Icon;
use crate::app::components::chat_panel::utils::{icon_svg, single_line_preview};

pub(crate) const DETAILS_SCROLLBAR_WIDTH: f32 = styles::DETAILS_SCROLLBAR_WIDTH;
const INPUT_PREVIEW_MAX_CHARS: usize = 80;
const INPUT_TOOLTIP_MAX_WIDTH: f32 = 420.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TrajectoryDetailField {
    pub(crate) label: &'static str,
    pub(crate) value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TrajectoryDetailsModel {
    pub(crate) summary: Vec<TrajectoryDetailField>,
    pub(crate) preview: Vec<TrajectoryDetailField>,
    pub(crate) raw: String,
}

impl TrajectoryDetailsModel {
    pub(crate) fn summary_value(&self, label: &str) -> Option<&str> {
        field_value(&self.summary, label)
    }

    pub(crate) fn preview_value(&self, label: &str) -> Option<&str> {
        field_value(&self.preview, label)
    }
}

pub(crate) fn build_trajectory_details(record: &TrajectoryRecord) -> TrajectoryDetailsModel {
    let summary = vec![
        field("Source", source(record)),
        field("Status", status(record)),
        field("Tokens", format_usage(record.usage)),
        field("Reasoning", known_text(record.detail.reasoning.as_deref())),
        field("Content", summary_content(record)),
        field("Started", format_started(record.started_ms)),
        field("Total duration", format_duration(record.metrics.duration_ms)),
    ];
    let preview = match record.kind {
        TrajectoryRecordKind::System | TrajectoryRecordKind::User => {
            vec![field("Input", known_trimmed_text(record.detail.input.as_deref()))]
        }
        TrajectoryRecordKind::Assistant => vec![
            field("Reasoning", known_text(record.detail.reasoning.as_deref())),
            field("Content", known_text(record.detail.output.as_deref())),
        ],
        TrajectoryRecordKind::Tool => vec![
            field("Tool", known_text(record.detail.tool_name.as_deref())),
            field("Call ID", known_text(record.detail.call_id.as_deref())),
            field("Arguments", known_text(record.detail.arguments.as_deref())),
            field("Result", known_text(record.detail.result.as_deref())),
        ],
    };
    let raw = pretty_raw(&record.detail.raw_fragments);
    TrajectoryDetailsModel { summary, preview, raw }
}

pub(crate) fn trajectory_details<'a, Message: Clone + 'a + 'static>(
    record: &TrajectoryRecord,
    active_tab: TrajectoryDetailsTab,
    on_tab: impl Fn(TrajectoryDetailsTab) -> Message,
    on_close: Message,
) -> Element<'a, Message> {
    let details = build_trajectory_details(record);
    let header = row![
        column![
            text("详情").size(15),
            text(format!("{} · #{}", source(record), record.index)).size(11).style(
                |theme: &Theme| iced::widget::text::Style {
                    color: Some(styles::details_label_color(theme)),
                }
            ),
        ]
        .spacing(2)
        .width(Length::Fill),
        button(icon_svg(Icon::X).width(Length::Fixed(12.0)).height(Length::Fixed(12.0)).style(
            |theme: &Theme, _status| iced::widget::svg::Style {
                color: Some(theme.palette().text.scale_alpha(0.78)),
            }
        ),)
        .padding(7)
        .style(styles::details_close_button_style)
        .on_press(on_close),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let mut tabs = row![].spacing(4);
    for tab in TrajectoryDetailsTab::ALL {
        let selected = active_tab == tab;
        tabs = tabs.push(
            button(text(tab.label()).size(11))
                .padding([5, 8])
                .style(move |theme, status| styles::details_tab_style(theme, status, selected))
                .on_press(on_tab(tab)),
        );
    }

    let body = match active_tab {
        TrajectoryDetailsTab::Summary => fields_view(details.summary, false),
        TrajectoryDetailsTab::Preview => fields_view(details.preview, true),
        TrajectoryDetailsTab::Raw => raw_view(details.raw),
    };
    let body = scrollable(container(body).width(Length::Fill).padding([2, 4]))
        .direction(details_scroll_direction())
        .width(Length::Fill)
        .height(Length::Fill);

    container(column![header, tabs, body].spacing(9).height(Length::Fill))
        .padding(10)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(styles::details_panel_style)
        .into()
}

pub(crate) fn details_scroll_direction() -> Direction {
    styles::details_scroll_direction()
}

pub(crate) fn detail_payload_font(payload: &str) -> Font {
    if payload.is_ascii() { Font::MONOSPACE } else { Font::DEFAULT }
}

pub(crate) const fn detail_payload_wrapping() -> Wrapping {
    Wrapping::WordOrGlyph
}

fn fields_view<'a, Message: 'a>(
    fields: Vec<TrajectoryDetailField>,
    expanded: bool,
) -> Element<'a, Message> {
    let mut content = column![].spacing(if expanded { 8 } else { 5 }).width(Length::Fill);
    for field in fields {
        content = if expanded {
            content.push(preview_field(field))
        } else {
            content.push(summary_field(field))
        };
    }
    content.into()
}

fn summary_field<'a, Message: 'a>(field: TrajectoryDetailField) -> Element<'a, Message> {
    row![
        text(field.label).size(11).width(Length::Fixed(104.0)).style(|theme: &Theme| {
            iced::widget::text::Style { color: Some(styles::details_label_color(theme)) }
        }),
        text(field.value).size(12).width(Length::Fill).wrapping(detail_payload_wrapping()),
    ]
    .spacing(8)
    .align_y(Alignment::Start)
    .into()
}

fn preview_field<'a, Message: 'a>(field: TrajectoryDetailField) -> Element<'a, Message> {
    if field.label == "Input" {
        return input_preview_field(field);
    }

    let use_monospace = matches!(field.label, "Tool" | "Call ID" | "Arguments");
    let font = if use_monospace { detail_payload_font(&field.value) } else { Font::DEFAULT };
    container(
        column![
            text(field.label).size(11).style(|theme: &Theme| iced::widget::text::Style {
                color: Some(styles::details_label_color(theme)),
            }),
            text(field.value)
                .font(font)
                .size(12)
                .width(Length::Fill)
                .wrapping(detail_payload_wrapping()),
        ]
        .spacing(4),
    )
    .padding(8)
    .width(Length::Fill)
    .style(styles::details_section_style)
    .into()
}

fn input_preview_field<'a, Message: 'a>(field: TrajectoryDetailField) -> Element<'a, Message> {
    let preview = input_preview_text(&field.value);
    let tooltip = container(
        text(field.value)
            .size(12)
            .width(Length::Fixed(INPUT_TOOLTIP_MAX_WIDTH))
            .wrapping(detail_payload_wrapping()),
    )
    .padding([8, 10])
    .max_width(INPUT_TOOLTIP_MAX_WIDTH)
    .style(styles::details_tooltip_style);

    container(
        column![
            text(field.label).size(11).style(|theme: &Theme| iced::widget::text::Style {
                color: Some(styles::details_label_color(theme)),
            }),
            Tooltip::new(
                text(preview).size(12).width(Length::Fill).wrapping(Wrapping::None),
                tooltip,
                TooltipPosition::Top,
            )
            .gap(6),
        ]
        .spacing(4),
    )
    .padding(8)
    .width(Length::Fill)
    .style(styles::details_section_style)
    .into()
}

pub(crate) fn input_preview_text(input: &str) -> String {
    single_line_preview(input, INPUT_PREVIEW_MAX_CHARS)
}

fn raw_view<'a, Message: 'a>(raw: String) -> Element<'a, Message> {
    let font = detail_payload_font(&raw);
    container(text(raw).font(font).size(11).width(Length::Fill).wrapping(detail_payload_wrapping()))
        .padding(8)
        .width(Length::Fill)
        .style(styles::details_section_style)
        .into()
}

fn field(label: &'static str, value: String) -> TrajectoryDetailField {
    TrajectoryDetailField { label, value }
}

fn field_value<'a>(fields: &'a [TrajectoryDetailField], label: &str) -> Option<&'a str> {
    fields.iter().find(|field| field.label == label).map(|field| field.value.as_str())
}

fn source(record: &TrajectoryRecord) -> String {
    match record.kind {
        TrajectoryRecordKind::System => "System".to_string(),
        TrajectoryRecordKind::User => "User".to_string(),
        TrajectoryRecordKind::Assistant => "Assistant".to_string(),
        TrajectoryRecordKind::Tool => record
            .detail
            .tool_name
            .as_deref()
            .map_or_else(|| "Tool".to_string(), |tool_name| format!("Tool · {tool_name}")),
    }
}

fn status(record: &TrajectoryRecord) -> String {
    if record.finished_ms.is_some()
        || (record.kind == TrajectoryRecordKind::Tool && record.detail.result.is_some())
    {
        "Completed".to_string()
    } else if record.started_ms.is_some() {
        "Running".to_string()
    } else {
        "—".to_string()
    }
}

fn summary_content(record: &TrajectoryRecord) -> String {
    let content = match record.kind {
        TrajectoryRecordKind::System | TrajectoryRecordKind::User => record.detail.input.as_deref(),
        TrajectoryRecordKind::Assistant => record.detail.output.as_deref(),
        TrajectoryRecordKind::Tool => record.detail.result.as_deref(),
    };
    known_text(content)
}

fn known_text(value: Option<&str>) -> String {
    value.filter(|value| !value.trim().is_empty()).unwrap_or("—").to_string()
}

fn known_trimmed_text(value: Option<&str>) -> String {
    value.map(str::trim).filter(|value| !value.is_empty()).unwrap_or("—").to_string()
}

fn format_usage(usage: MetricAvailability<TrajectoryUsage>) -> String {
    match usage {
        MetricAvailability::Available(usage) => format!(
            "input {} · output {} · cached {} · reasoning {}",
            usage.input_tokens, usage.output_tokens, usage.cached_tokens, usage.reasoning_tokens
        ),
        MetricAvailability::Unavailable => "—".to_string(),
    }
}

fn format_started(started_ms: Option<u64>) -> String {
    started_ms.map_or_else(|| "—".to_string(), |started_ms| format!("{started_ms} ms"))
}

fn format_duration(duration_ms: MetricAvailability<u64>) -> String {
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

fn pretty_raw(fragments: &[serde_json::Value]) -> String {
    match fragments {
        [] => "—".to_string(),
        [fragment] => serde_json::to_string_pretty(fragment).unwrap_or_else(|_| "—".to_string()),
        fragments => serde_json::to_string_pretty(fragments).unwrap_or_else(|_| "—".to_string()),
    }
}
