//! 轨迹详情正文的纯模型。

use serde_json::Value;

use iced::widget::text::Wrapping;
use iced::widget::{column, container, row, text, text_editor};
use iced::{Alignment, Element, Font, Length, Theme};

use super::{
    MetricAvailability, TrajectoryDetailsState, TrajectoryDetailsTab, TrajectoryRecord,
    TrajectoryRecordKind, TrajectoryUsage,
};
use super::{json_tree, styles};
use crate::app::components::chat_panel::tool_text_support::{
    READ_ONLY_EDITOR_MAX_HEIGHT, read_only_text_style,
};
use crate::app::components::text_editor_context_menu::{
    CopyTextContextMenuMessages, TextEditorContextMenuState, wrap_with_copy_context_menu,
};
use crate::app::components::text_editor_scroll_panel::pass_wheel_to_parent;
use crate::app::{Message, message};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum TrajectoryDetailContent {
    Fields(Vec<(String, String)>),
    Text(String),
    Json(Value),
    Unavailable(String),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TrajectoryDetailsContentModel {
    pub(crate) badge: String,
    pub(crate) location: String,
    pub(crate) tabs: Vec<TrajectoryDetailsTab>,
    pub(crate) body: TrajectoryDetailContent,
}

pub(crate) fn build_details_content(
    record: &TrajectoryRecord,
    tab: TrajectoryDetailsTab,
) -> TrajectoryDetailsContentModel {
    TrajectoryDetailsContentModel {
        badge: record_badge(record.kind).to_string(),
        location: record_location(record),
        tabs: record.kind.detail_tabs().to_vec(),
        body: content_for_tab(record, tab),
    }
}

pub(crate) fn detail_content_values(body: &TrajectoryDetailContent) -> Vec<String> {
    match body {
        TrajectoryDetailContent::Fields(fields) => {
            fields.iter().map(|(_, value)| value.clone()).collect()
        }
        TrajectoryDetailContent::Text(text) | TrajectoryDetailContent::Unavailable(text) => {
            vec![text.clone()]
        }
        TrajectoryDetailContent::Json(value) => vec![
            serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string()),
        ],
    }
}

pub(crate) fn trajectory_details_body<'a>(
    body: TrajectoryDetailContent,
    state: &'a TrajectoryDetailsState,
) -> Element<'a, Message> {
    match body {
        TrajectoryDetailContent::Fields(fields) => fields_view(fields, state),
        TrajectoryDetailContent::Text(value) => container(selectable_value(
            0,
            value,
            Font::DEFAULT,
            12.0,
            state,
        ))
        .padding(10)
        .width(Length::Fill)
        .style(styles::details_section_style)
        .into(),
        TrajectoryDetailContent::Json(value) => {
            let collapsed_paths = state.json_collapsed_paths();
            json_tree::json_tree(
                &value,
                &collapsed_paths,
                |path| {
                    Message::Chat(message::ChatMessage::TrajectoryDetailsJsonToggled(path))
                },
                |value| {
                    Message::Chat(message::ChatMessage::TrajectoryDetailsJsonCopied(value))
                },
            )
        }
        TrajectoryDetailContent::Unavailable(value) => text(value)
            .font(Font::DEFAULT)
            .size(12)
            .style(|theme: &Theme| iced::widget::text::Style {
                color: Some(styles::empty_text_color(theme)),
            })
            .into(),
    }
}

fn fields_view<'a>(
    fields: Vec<(String, String)>,
    state: &'a TrajectoryDetailsState,
) -> Element<'a, Message> {
    let mut content = column![].spacing(7).width(Length::Fill);
    for (index, (label, value)) in fields.into_iter().enumerate() {
        content = content.push(
            row![
                text(label).font(Font::DEFAULT).size(11).width(Length::Fixed(112.0)).style(
                    |theme: &Theme| iced::widget::text::Style {
                        color: Some(styles::details_label_color(theme)),
                    }
                ),
                selectable_value(index, value, Font::DEFAULT, 12.0, state),
            ]
            .spacing(8)
            .align_y(Alignment::Start),
        );
    }
    content.into()
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
            .wrapping(Wrapping::WordOrGlyph)
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

fn record_badge(kind: TrajectoryRecordKind) -> &'static str {
    match kind {
        TrajectoryRecordKind::System => "SYSTEM",
        TrajectoryRecordKind::User => "USER",
        TrajectoryRecordKind::Context => "CONTEXT",
        TrajectoryRecordKind::Assistant => "MODEL",
        TrajectoryRecordKind::Tool => "TOOL",
    }
}

fn record_location(record: &TrajectoryRecord) -> String {
    let item = match record.kind {
        TrajectoryRecordKind::System => {
            return format!("Step {} · Record #{}", record.step_index, record.index);
        }
        TrajectoryRecordKind::User | TrajectoryRecordKind::Context => "Message",
        TrajectoryRecordKind::Assistant => "Model",
        TrajectoryRecordKind::Tool => "Tool",
    };
    record.turn_index.map_or_else(
        || format!("Step {} · {item}", record.step_index),
        |turn| format!("Turn {turn} · {item}"),
    )
}

fn content_for_tab(
    record: &TrajectoryRecord,
    tab: TrajectoryDetailsTab,
) -> TrajectoryDetailContent {
    match (record.kind, tab) {
        (TrajectoryRecordKind::System, TrajectoryDetailsTab::SystemPrompt) => record
            .detail
            .input
            .as_deref()
            .filter(|prompt| !prompt.trim().is_empty())
            .map(|prompt| TrajectoryDetailContent::Text(prompt.to_string()))
            .unwrap_or_else(|| unavailable("No system prompt in this request")),
        (TrajectoryRecordKind::System, TrajectoryDetailsTab::Tools) => {
            if record.detail.request_tools.is_empty() {
                unavailable("No tools in this request")
            } else {
                TrajectoryDetailContent::Json(Value::Array(
                    record
                        .detail
                        .request_tools
                        .iter()
                        .map(|tool| {
                            serde_json::json!({
                                "id": tool.id,
                                "description": tool.description,
                                "input_schema": tool.input_schema,
                            })
                        })
                        .collect(),
                ))
            }
        }
        (
            TrajectoryRecordKind::User | TrajectoryRecordKind::Context,
            TrajectoryDetailsTab::Summary,
        ) => TrajectoryDetailContent::Fields(vec![
            ("Source".to_string(), message_source_label(record)),
            ("Status".to_string(), status(record)),
            ("Duration".to_string(), format_duration(record.metrics.duration_ms)),
        ]),
        (
            TrajectoryRecordKind::User | TrajectoryRecordKind::Context,
            TrajectoryDetailsTab::Preview,
        ) => detail_text(record.detail.input.as_deref(), "No content"),
        (
            TrajectoryRecordKind::User | TrajectoryRecordKind::Context,
            TrajectoryDetailsTab::Raw,
        ) => raw_fragments(record),
        (
            TrajectoryRecordKind::User | TrajectoryRecordKind::Context,
            TrajectoryDetailsTab::Source,
        ) => record
            .detail
            .source
            .clone()
            .map(TrajectoryDetailContent::Json)
            .unwrap_or_else(|| unavailable("Source not recorded")),
        (TrajectoryRecordKind::Assistant, TrajectoryDetailsTab::Summary) => {
            let mut fields = vec![
                ("Status".to_string(), status(record)),
                ("Tokens".to_string(), format_usage(record.usage)),
            ];
            if let Some(reasoning) = nonempty(record.detail.reasoning.as_deref()) {
                fields.push(("Reasoning".to_string(), reasoning.to_string()));
            }
            fields.push((
                "Content".to_string(),
                nonempty(record.detail.output.as_deref()).unwrap_or("—").to_string(),
            ));
            TrajectoryDetailContent::Fields(fields)
        }
        (TrajectoryRecordKind::Assistant, TrajectoryDetailsTab::Options) => record
            .detail
            .request_options
            .clone()
            .map(TrajectoryDetailContent::Json)
            .unwrap_or_else(|| unavailable("Options not recorded")),
        (TrajectoryRecordKind::Assistant, TrajectoryDetailsTab::Usage) => match record.usage {
            MetricAvailability::Available(usage) => usage_content(usage),
            MetricAvailability::Unavailable => unavailable("Usage not recorded"),
        },
        (TrajectoryRecordKind::Assistant, TrajectoryDetailsTab::Timing) => timing_content(record),
        (TrajectoryRecordKind::Tool, TrajectoryDetailsTab::Summary) => {
            TrajectoryDetailContent::Fields(vec![
                ("Status".to_string(), status(record)),
                (
                    "Tool".to_string(),
                    nonempty(record.detail.tool_name.as_deref()).unwrap_or("—").to_string(),
                ),
                (
                    "Call ID".to_string(),
                    nonempty(record.detail.call_id.as_deref()).unwrap_or("—").to_string(),
                ),
            ])
        }
        (TrajectoryRecordKind::Tool, TrajectoryDetailsTab::Payload) => {
            parsed_or_text(record.detail.arguments.as_deref(), "No payload captured")
        }
        (TrajectoryRecordKind::Tool, TrajectoryDetailsTab::Result) => {
            parsed_or_text(record.detail.result.as_deref(), "No result captured")
        }
        (TrajectoryRecordKind::Tool, TrajectoryDetailsTab::Schema) => record
            .detail
            .tool_schema
            .clone()
            .map(TrajectoryDetailContent::Json)
            .unwrap_or_else(|| unavailable("Schema unavailable")),
        (TrajectoryRecordKind::Tool, TrajectoryDetailsTab::Timing) => timing_content(record),
        _ => unavailable("This tab is not available for this record"),
    }
}

fn raw_fragments(record: &TrajectoryRecord) -> TrajectoryDetailContent {
    match record.detail.raw_fragments.as_slice() {
        [] => unavailable("Raw payload not recorded"),
        [fragment] => TrajectoryDetailContent::Json(fragment.clone()),
        fragments => TrajectoryDetailContent::Json(Value::Array(fragments.to_vec())),
    }
}

fn parsed_or_text(value: Option<&str>, missing: &'static str) -> TrajectoryDetailContent {
    let Some(value) = nonempty(value) else {
        return unavailable(missing);
    };
    serde_json::from_str(value)
        .map(TrajectoryDetailContent::Json)
        .unwrap_or_else(|_| TrajectoryDetailContent::Text(value.to_string()))
}

fn detail_text(value: Option<&str>, missing: &'static str) -> TrajectoryDetailContent {
    nonempty(value)
        .map(|value| TrajectoryDetailContent::Text(value.to_string()))
        .unwrap_or_else(|| unavailable(missing))
}

fn usage_content(usage: TrajectoryUsage) -> TrajectoryDetailContent {
    TrajectoryDetailContent::Fields(vec![
        ("Input tokens".to_string(), usage.input_tokens.to_string()),
        ("Output tokens".to_string(), usage.output_tokens.to_string()),
        ("Cached tokens".to_string(), usage.cached_tokens.to_string()),
        ("Reasoning tokens".to_string(), usage.reasoning_tokens.to_string()),
    ])
}

fn timing_content(record: &TrajectoryRecord) -> TrajectoryDetailContent {
    let duration = if record.kind == TrajectoryRecordKind::Tool {
        record.metrics.tool_duration_ms
    } else {
        record.metrics.duration_ms
    };
    TrajectoryDetailContent::Fields(vec![
        (
            "Started".to_string(),
            record.started_ms.map_or_else(|| "—".to_string(), |value| format!("{value} ms")),
        ),
        ("Duration".to_string(), format_duration(duration)),
        (
            "Timing source".to_string(),
            if matches!(duration, MetricAvailability::Available(_)) {
                "Session timestamps".to_string()
            } else {
                "Not available".to_string()
            },
        ),
    ])
}

fn message_source_label(record: &TrajectoryRecord) -> String {
    let Some(source) = record.detail.source.as_ref() else {
        return "Not recorded".to_string();
    };
    let Some(kind) = source.get("kind").and_then(Value::as_str).filter(|kind| !kind.is_empty())
    else {
        return "Unknown".to_string();
    };
    if kind == "user" {
        return "User".to_string();
    }
    kind.split(['_', '-'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(chars).collect::<String>()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
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

fn format_usage(usage: MetricAvailability<TrajectoryUsage>) -> String {
    match usage {
        MetricAvailability::Available(usage) => format!(
            "input {} · output {} · cached {} · reasoning {}",
            usage.input_tokens, usage.output_tokens, usage.cached_tokens, usage.reasoning_tokens
        ),
        MetricAvailability::Unavailable => "—".to_string(),
    }
}

fn format_duration(duration: MetricAvailability<u64>) -> String {
    match duration {
        MetricAvailability::Available(duration) if duration < 1_000 => format!("{duration} ms"),
        MetricAvailability::Available(duration) => {
            format!("{}.{:01} s", duration / 1_000, duration % 1_000 / 100)
        }
        MetricAvailability::Unavailable => "—".to_string(),
    }
}

fn nonempty(value: Option<&str>) -> Option<&str> {
    value.filter(|value| !value.trim().is_empty())
}

fn unavailable(message: &'static str) -> TrajectoryDetailContent {
    TrajectoryDetailContent::Unavailable(message.to_string())
}
