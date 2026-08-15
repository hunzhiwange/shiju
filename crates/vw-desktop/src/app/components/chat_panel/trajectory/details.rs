//! 选中轨迹记录的 Summary、Preview 与 Raw 检查器。

use iced::widget::scrollable::Direction;
use iced::widget::text::Wrapping;
use iced::widget::{Space, button, column, container, row, scrollable, text, text_editor};
use iced::{Alignment, Element, Font, Length, Theme};

use super::details_content::{
    build_details_content, detail_content_values, trajectory_details_body,
};
use super::styles;
use super::{
    MetricAvailability, TrajectoryDetailsState, TrajectoryDetailsTab, TrajectoryRecord,
    TrajectoryRecordKind, TrajectoryUsage,
};
use crate::app::assets::Icon;
use crate::app::components::chat_panel::tool_text_support::{
    READ_ONLY_EDITOR_MAX_HEIGHT, read_only_text_style,
};
use crate::app::components::chat_panel::utils::{icon_svg, single_line_preview};
use crate::app::components::text_editor_context_menu::{
    CopyTextContextMenuMessages, TextEditorContextMenuState, wrap_with_copy_context_menu,
};
use crate::app::components::text_editor_scroll_panel::pass_wheel_to_parent;
use crate::app::{Message, message};

pub(crate) const DETAILS_SCROLLBAR_WIDTH: f32 = styles::DETAILS_SCROLLBAR_WIDTH;
const INPUT_PREVIEW_MAX_CHARS: usize = 80;

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
        TrajectoryRecordKind::System
        | TrajectoryRecordKind::User
        | TrajectoryRecordKind::Context => {
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

pub(crate) fn trajectory_detail_values(
    record: &TrajectoryRecord,
    active_tab: TrajectoryDetailsTab,
) -> Vec<String> {
    detail_content_values(&build_details_content(record, active_tab).body)
}

pub(crate) fn trajectory_details<'a>(
    record: &TrajectoryRecord,
    state: &'a TrajectoryDetailsState,
) -> Element<'a, Message> {
    let active_tab = state.active_tab();
    let record_kind = record.kind;
    let details = build_details_content(record, active_tab);
    let header = row![
        container(text(details.badge).font(Font::DEFAULT).size(10))
            .padding([3, 6])
            .style(move |theme: &Theme| styles::role_badge_style(theme, record_kind)),
        text(details.location).font(Font::DEFAULT).size(12).width(Length::Fill).style(
            |theme: &Theme| iced::widget::text::Style {
                color: Some(styles::details_label_color(theme)),
            }
        ),
        button(icon_svg(Icon::X).width(Length::Fixed(12.0)).height(Length::Fixed(12.0)).style(
            |theme: &Theme, _status| iced::widget::svg::Style {
                color: Some(theme.palette().text.scale_alpha(0.78)),
            }
        ),)
        .padding(7)
        .style(styles::details_close_button_style)
        .on_press(Message::Chat(message::ChatMessage::TrajectoryDetailsClosed)),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let mut tabs = row![].spacing(4);
    for tab in details.tabs {
        let selected = active_tab == tab;
        let tab_button = button(text(tab.label()).font(Font::DEFAULT).size(11))
            .padding([5, 8])
            .style(move |theme, status| styles::details_tab_style(theme, status, selected))
            .on_press(Message::Chat(message::ChatMessage::TrajectoryDetailsTabSelected(tab)));
        let underline = container(Space::new().height(Length::Fixed(2.0)))
            .width(Length::Fill)
            .height(Length::Fixed(2.0))
            .style(move |theme: &Theme| styles::details_tab_underline_style(theme, selected));
        tabs = tabs.push(column![tab_button, underline].spacing(1));
    }

    let body = trajectory_details_body(details.body, state);
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

fn fields_view<'a>(
    fields: Vec<TrajectoryDetailField>,
    expanded: bool,
    state: &'a TrajectoryDetailsState,
) -> Element<'a, Message> {
    let mut content = column![].spacing(if expanded { 8 } else { 5 }).width(Length::Fill);
    for (index, field) in fields.into_iter().enumerate() {
        content = if expanded {
            content.push(preview_field(index, field, state))
        } else {
            content.push(summary_field(index, field, state))
        };
    }
    content.into()
}

fn summary_field<'a>(
    index: usize,
    field: TrajectoryDetailField,
    state: &'a TrajectoryDetailsState,
) -> Element<'a, Message> {
    row![
        text(field.label).size(11).width(Length::Fixed(104.0)).style(|theme: &Theme| {
            iced::widget::text::Style { color: Some(styles::details_label_color(theme)) }
        }),
        selectable_value(index, field.value, Font::DEFAULT, 12.0, state),
    ]
    .spacing(8)
    .align_y(Alignment::Start)
    .into()
}

fn preview_field<'a>(
    index: usize,
    field: TrajectoryDetailField,
    state: &'a TrajectoryDetailsState,
) -> Element<'a, Message> {
    let use_monospace = matches!(field.label, "Tool" | "Call ID" | "Arguments");
    let font = if use_monospace { detail_payload_font(&field.value) } else { Font::DEFAULT };
    container(
        column![
            text(field.label).size(11).style(|theme: &Theme| iced::widget::text::Style {
                color: Some(styles::details_label_color(theme)),
            }),
            selectable_value(index, field.value, font, 12.0, state),
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

fn raw_view<'a>(raw: String, state: &'a TrajectoryDetailsState) -> Element<'a, Message> {
    let font = detail_payload_font(&raw);
    container(selectable_value(0, raw, font, 11.0, state))
        .padding(8)
        .width(Length::Fill)
        .style(styles::details_section_style)
        .into()
}

fn selectable_value<'a>(
    index: usize,
    value: String,
    font: Font,
    size: f32,
    state: &'a TrajectoryDetailsState,
) -> Element<'a, Message> {
    let content: Element<'a, Message> = if let Some(editor_content) = state.editor(index) {
        let editor = text_editor(editor_content)
            .on_action(move |action| {
                Message::Chat(message::ChatMessage::TrajectoryDetailsEditorAction(index, action))
            })
            .size(size)
            .padding(0)
            .height(Length::Shrink)
            .max_height(READ_ONLY_EDITOR_MAX_HEIGHT)
            .font(font)
            .style(|theme: &Theme, _status| read_only_text_style(theme, theme.palette().text));
        container(pass_wheel_to_parent(editor)).width(Length::Fill).into()
    } else {
        text(value)
            .font(font)
            .size(size)
            .width(Length::Fill)
            .wrapping(detail_payload_wrapping())
            .into()
    };
    let position = state.context_menu_position(index);

    wrap_with_copy_context_menu(
        content,
        TextEditorContextMenuState { open: position.is_some(), position },
        move |point| {
            Message::Chat(message::ChatMessage::TrajectoryDetailsContextMenuOpened {
                index,
                x: point.x,
                y: point.y,
            })
        },
        CopyTextContextMenuMessages {
            close: Message::Chat(message::ChatMessage::TrajectoryDetailsContextMenuClosed),
            copy: Message::Chat(message::ChatMessage::TrajectoryDetailsContextMenuCopy),
        },
    )
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
        TrajectoryRecordKind::Context => "Context".to_string(),
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
        TrajectoryRecordKind::System
        | TrajectoryRecordKind::User
        | TrajectoryRecordKind::Context => record.detail.input.as_deref(),
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
