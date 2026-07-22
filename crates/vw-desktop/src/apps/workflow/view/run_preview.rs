//! Workflow saved app run preview overlay.

use super::*;
use chrono::{Local, TimeZone};
use iced::widget::{column, row};
use state::{WorkflowRunNodePreview, WorkflowRunPreview, WorkflowRunStatus};

const RUN_PREVIEW_WIDTH: f32 = 860.0;
const RUN_PREVIEW_MAX_HEIGHT: f32 = 760.0;
pub(super) const RUN_PREVIEW_SCROLLBAR_WIDTH: f32 = 4.0;
const RUN_PREVIEW_SCROLL_GUTTER: f32 = 12.0;
const RUN_PREVIEW_NODE_LABEL_WIDTH: f32 = 74.0;
const RUN_PREVIEW_NODE_RAIL_WIDTH: f32 = 12.0;
const RUN_PREVIEW_NODE_MAX_FIELDS: usize = 8;
const RUN_PREVIEW_NODE_MAX_VALUE_CHARS: usize = 96;
pub(super) const RUN_PREVIEW_PENDING_RUN_ID_LABEL: &str = "等待运行 ID";
const RUN_PREVIEW_EMPTY_TIMELINE_LABEL: &str = "等待节点事件...";

pub(super) fn saved_app_run_preview_close_message() -> WorkflowMessage {
    WorkflowMessage::CloseSavedAppRunPreview
}

/// Resolves the summary's run-id label, falling back to a placeholder before the
/// gateway assigns a run id so the header stays stable for a freshly started run.
pub(super) fn run_preview_run_id_label(preview: &WorkflowRunPreview) -> &str {
    preview.run_id.as_deref().unwrap_or(RUN_PREVIEW_PENDING_RUN_ID_LABEL)
}

pub(super) fn saved_app_run_preview_action(
    preview: &WorkflowRunPreview,
) -> Option<(&'static str, WorkflowMessage)> {
    match (preview.is_finished, preview.status) {
        (false, WorkflowRunStatus::Running) => {
            Some(("取消运行", WorkflowMessage::CancelSavedAppRun))
        }
        (true, WorkflowRunStatus::Succeeded | WorkflowRunStatus::Failed) => {
            Some(("重新执行", WorkflowMessage::RetrySavedAppRunPreview))
        }
        _ => None,
    }
}

pub(super) fn build_saved_app_run_preview_layer(
    preview: &WorkflowRunPreview,
    error_copied: bool,
) -> Element<'_, Message> {
    let close_message = Message::WorkflowTool(saved_app_run_preview_close_message());
    let run_id_label = run_preview_run_id_label(preview);
    let preview_action = saved_app_run_preview_action(preview);
    let summary = row![
        workflow_run_status_badge(preview.status),
        text(format!("UUID: {}", preview.uuid)).size(12).style(settings_muted_text_style),
        text(format!("Run: {run_id_label}")).size(12).style(settings_muted_text_style),
    ]
    .spacing(10)
    .wrap();

    let mut timeline = column![].spacing(8).width(Length::Fill);
    if preview.nodes.is_empty() {
        timeline = timeline.push(
            row![
                Space::new().width(RUN_PREVIEW_NODE_RAIL_WIDTH),
                container(
                    text(RUN_PREVIEW_EMPTY_TIMELINE_LABEL)
                        .size(12)
                        .style(settings_muted_text_style)
                )
                .padding([12, 14])
                .width(Length::Fill)
                .style(workflow_run_preview_node_style),
            ]
            .spacing(8)
            .width(Length::Fill),
        );
    } else {
        for (index, node) in preview.nodes.iter().enumerate() {
            timeline = timeline.push(workflow_run_node_row(index, node));
        }
    }

    let output_action = workflow_run_output_text(preview.text_output.as_str()).map(|_| {
        run_preview_detail_button(Message::WorkflowTool(
            WorkflowMessage::OpenSavedAppRunOutputDetail,
        ))
    });

    let mut body = column![
        run_preview_section("节点时间线", None, timeline.into()),
        run_preview_section(
            "输出",
            output_action,
            workflow_run_output(preview.text_output.as_str()),
        ),
    ]
    .spacing(16)
    .width(Length::Fill);

    if preview.error.is_some() {
        body = body.push(run_preview_section(
            "错误详情",
            Some(run_preview_copy_button(
                error_copied,
                Message::WorkflowTool(WorkflowMessage::CopySavedAppRunError),
            )),
            workflow_run_error(preview),
        ));
    }

    let scroll_body = scrollable(container(body).width(Length::Fill).padding(iced::Padding {
        top: 0.0,
        right: RUN_PREVIEW_SCROLLBAR_WIDTH + RUN_PREVIEW_SCROLL_GUTTER,
        bottom: 0.0,
        left: 0.0,
    }))
    .direction(Direction::Vertical(
        Scrollbar::new()
            .width(RUN_PREVIEW_SCROLLBAR_WIDTH)
            .scroller_width(RUN_PREVIEW_SCROLLBAR_WIDTH),
    ))
    .height(Length::Fill);

    let mut header_actions = row![].spacing(10).align_y(Alignment::Center);
    if let Some((label, action)) = preview_action {
        let button_style = match action {
            WorkflowMessage::CancelSavedAppRun => danger_action_btn_style,
            WorkflowMessage::RetrySavedAppRunPreview => primary_action_btn_style,
            _ => rounded_action_btn_style,
        };
        header_actions = header_actions.push(
            button(text(label).size(13))
                .style(button_style)
                .padding([8, 14])
                .on_press(Message::WorkflowTool(action)),
        );
    }
    header_actions = header_actions.push(settings_close_button(close_message.clone()));

    let card = container(
        column![
            row![
                column![
                    text("运行预览").size(18).line_height(1.0),
                    text("实时展示 Workflow 执行过程").size(12).style(settings_muted_text_style),
                ]
                .spacing(6)
                .width(Length::Fill),
                header_actions,
            ]
            .spacing(12)
            .align_y(Alignment::Center),
            summary,
            scroll_body,
        ]
        .spacing(18)
        .width(Length::Fill),
    )
    .width(Length::Fixed(RUN_PREVIEW_WIDTH))
    .height(Length::Shrink)
    .max_height(RUN_PREVIEW_MAX_HEIGHT)
    .padding([22, 24])
    .style(workflow_run_preview_card_style);

    settings_modal_overlay(None, close_message, card)
}

/// 构建运行预览的“查看详细”弹窗：以只读、可滚动、可选中复制的编辑器展示完整内容。
pub(super) fn build_saved_app_run_detail_layer(
    detail: &state::WorkflowRunDetailDialog,
) -> Element<'_, Message> {
    let close_message = Message::WorkflowTool(WorkflowMessage::CloseSavedAppRunDetail);

    let editor = text_editor(&detail.content)
        .size(13)
        .padding([10, 12])
        .height(Length::Fill)
        .on_action(|action| Message::WorkflowTool(WorkflowMessage::SavedAppRunDetailAction(action)))
        .style(workflow_run_detail_editor_style);

    let card = container(
        column![
            row![
                column![
                    text(&detail.title).size(16).line_height(1.1).width(Length::Fill),
                    text("可滚动并选中复制完整内容").size(12).style(settings_muted_text_style),
                ]
                .spacing(6)
                .width(Length::Fill),
                settings_close_button(close_message.clone()),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
            container(editor)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(4)
                .style(workflow_run_preview_node_style),
        ]
        .spacing(16)
        .width(Length::Fill),
    )
    .width(Length::Fixed(760.0))
    .height(Length::Fixed(560.0))
    .padding([22, 24])
    .style(workflow_run_preview_card_style);

    settings_modal_overlay(None, close_message, card)
}

fn workflow_run_detail_editor_style(
    theme: &Theme,
    _status: text_editor::Status,
) -> text_editor::Style {
    let palette = theme.extended_palette();
    text_editor::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 0.0.into() },
        value: theme.palette().text,
        selection: theme.palette().primary.scale_alpha(0.30),
        placeholder: palette.background.strong.color,
    }
}

/// Wraps a labeled section so its heading hugs its content; sections themselves are
/// spaced apart by the body column, giving the scroll area a clear visual hierarchy.
/// `action` adds a trailing control (e.g. a "查看详细" button) on the heading row.
fn run_preview_section<'a>(
    label: &'a str,
    action: Option<Element<'a, Message>>,
    content: Element<'a, Message>,
) -> Element<'a, Message> {
    let mut heading =
        row![text(label).size(13).style(settings_muted_text_style).width(Length::Fill),]
            .spacing(8)
            .align_y(Alignment::Center);
    if let Some(action) = action {
        heading = heading.push(action);
    }

    column![heading, content].spacing(8).width(Length::Fill).into()
}

/// 渲染运行预览里的“查看详细”按钮，复用次级按钮样式，与节点/输出标题对齐。
fn run_preview_detail_button(on_press: Message) -> Element<'static, Message> {
    button(text("查看详细").size(11))
        .style(rounded_action_btn_style)
        .padding([4, 10])
        .on_press(on_press)
        .into()
}

/// 渲染错误详情的复制按钮：复制后短暂显示 ✓，2 秒后由计时消息恢复为“复制”。
fn run_preview_copy_button(copied: bool, on_press: Message) -> Element<'static, Message> {
    let label = if copied { "✓" } else { "复制" };
    let mut control = button(text(label).size(11)).padding([4, 10]);
    control = if copied {
        control.style(run_preview_copied_button_style)
    } else {
        control.style(rounded_action_btn_style).on_press(on_press)
    };
    control.into()
}

/// 复制成功后的按钮样式：成功色描边与底色，让 ✓ 在暗色主题下保持足够对比。
fn run_preview_copied_button_style(
    theme: &Theme,
    _status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let color = theme.extended_palette().success.base.color;
    iced::widget::button::Style {
        background: Some(Background::Color(color.scale_alpha(if is_dark_theme(theme) {
            0.20
        } else {
            0.12
        }))),
        border: Border { width: 1.0, color: color.scale_alpha(0.48), radius: 6.0.into() },
        text_color: opaque_color(color),
        ..Default::default()
    }
}

fn workflow_run_status_badge(status: WorkflowRunStatus) -> Element<'static, Message> {
    let label = workflow_run_status_label(status);
    container(
        row![
            workflow_status_dot(move |theme| workflow_run_status_color(theme, status)),
            text(label).size(12).line_height(1.0),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .padding([6, 10])
    .style(move |theme: &Theme| workflow_run_status_badge_style(theme, status))
    .into()
}

fn workflow_run_node_row(index: usize, node: &WorkflowRunNodePreview) -> Element<'_, Message> {
    let status = workflow_run_node_status_label(node.status.as_deref());
    let status_kind = workflow_run_node_status_kind(node.status.as_deref());
    let data_preview = workflow_run_node_data_preview_items(node);

    // Left rail: a status-colored dot anchors each node onto the timeline.
    let rail = container(workflow_status_dot(move |theme| {
        workflow_run_status_kind_color(theme, status_kind)
    }))
    .width(Length::Fixed(RUN_PREVIEW_NODE_RAIL_WIDTH))
    .height(Length::Fill)
    .align_x(Alignment::Center)
    .padding([6, 0]);

    let header = row![
        text(node.title.as_str()).size(14).line_height(1.1).width(Length::Fill),
        run_preview_detail_button(Message::WorkflowTool(
            WorkflowMessage::OpenSavedAppRunNodeDetail(index),
        )),
        container(
            row![
                workflow_status_dot(move |theme| workflow_run_status_kind_color(
                    theme,
                    status_kind
                )),
                text(status).size(11).line_height(1.0),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .padding([4, 8])
        .style(move |theme| workflow_run_preview_chip_style(theme, status_kind)),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let card = container(
        column![
            header,
            text(format!("ID: {}", node.id)).size(11).style(settings_muted_text_style),
            workflow_run_node_details(data_preview),
        ]
        .spacing(6)
        .width(Length::Fill),
    )
    .padding([12, 14])
    .width(Length::Fill)
    .style(workflow_run_preview_node_style);

    row![rail, card].spacing(8).width(Length::Fill).into()
}

/// Normalizes the run's text output for display: trims outer whitespace and
/// reports `None` for empty output so callers can render a distinct empty state.
/// Inner content (including newlines) is preserved verbatim so long reports keep
/// their structure inside the scrollable card.
pub(super) fn workflow_run_output_text(output: &str) -> Option<String> {
    let trimmed = output.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

pub(super) fn workflow_run_output(output: &str) -> Element<'_, Message> {
    match workflow_run_output_text(output) {
        None => container(text("暂无文本输出").size(12).style(settings_muted_text_style))
            .padding([12, 14])
            .width(Length::Fill)
            .style(workflow_run_preview_output_empty_style)
            .into(),
        Some(text_output) => container(text(text_output).size(12))
            .padding([12, 14])
            .width(Length::Fill)
            .style(workflow_run_preview_output_style)
            .into(),
    }
}

pub(super) fn workflow_run_error(preview: &WorkflowRunPreview) -> Element<'_, Message> {
    let report = column![
        text("失败详情").size(12).line_height(1.0).style(workflow_run_preview_error_heading_style),
        text(preview.debug_text()).size(12),
    ]
    .spacing(8)
    .width(Length::Fill);

    container(report)
        .padding([12, 14])
        .width(Length::Fill)
        .style(workflow_run_preview_error_style)
        .into()
}

fn workflow_run_node_details(items: Vec<(String, String)>) -> Element<'static, Message> {
    if items.is_empty() {
        return text("暂无节点详情").size(11).style(settings_muted_text_style).into();
    }

    let mut details = column![].spacing(4).width(Length::Fill);
    for (label, value) in items {
        details = details.push(
            row![
                text(label)
                    .size(11)
                    .width(Length::Fixed(RUN_PREVIEW_NODE_LABEL_WIDTH))
                    .style(settings_muted_text_style),
                text(value).size(11).width(Length::Fill).style(settings_muted_text_style),
            ]
            .spacing(6)
            .width(Length::Fill),
        );
    }

    details.into()
}

pub(super) fn workflow_run_node_data_preview_items(
    node: &WorkflowRunNodePreview,
) -> Vec<(String, String)> {
    let mut items = Vec::new();
    let mut shown_keys = Vec::new();
    let Some(map) = node.data.as_object() else {
        return workflow_run_preview_value("data", &node.data)
            .map(|value| vec![("data".to_string(), value)])
            .unwrap_or_default();
    };

    for (key, label) in [
        ("node_type", "类型"),
        ("index", "序号"),
        ("predecessor_node_id", "上游"),
        ("created_at", "开始时间"),
        ("created_at_ms", "开始时间"),
        ("started_at", "开始时间"),
        ("started_at_ms", "开始时间"),
        ("finished_at", "完成时间"),
        ("finished_at_ms", "完成时间"),
        ("elapsed_time", "耗时"),
        ("elapsed_ms", "耗时"),
        ("duration_ms", "耗时"),
        ("error", "错误"),
        ("inputs", "输入"),
        ("process_data", "过程"),
        ("outputs", "输出"),
    ] {
        shown_keys.push(key.to_string());
        if let Some(value) = map.get(key).and_then(|value| workflow_run_preview_value(key, value)) {
            items.push((format!("{label}:"), value));
        }
    }

    for (key, value) in map {
        if items.len() >= RUN_PREVIEW_NODE_MAX_FIELDS {
            break;
        }
        if should_skip_run_preview_key(key) || shown_keys.iter().any(|shown| shown == key) {
            continue;
        }
        if let Some(value) = workflow_run_preview_value(key, value) {
            items.push((format!("{key}:"), value));
        }
    }

    items
}

fn should_skip_run_preview_key(key: &str) -> bool {
    matches!(key, "id" | "node_id" | "title" | "status") || is_sensitive_run_preview_key(key)
}

fn is_sensitive_run_preview_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    lower.starts_with("__vw_")
        || lower.contains("token")
        || lower.contains("secret")
        || lower.contains("password")
        || lower.contains("api_key")
        || lower.contains("apikey")
        || lower == "authorization"
        || lower == "cookie"
}

fn workflow_run_preview_value(key: &str, value: &serde_json::Value) -> Option<String> {
    if value.is_null() {
        return None;
    }

    if is_duration_run_preview_key(key) {
        if let Some(duration) = format_run_preview_duration(key, value) {
            return Some(duration);
        }
    }

    if is_time_run_preview_key(key) {
        return format_run_preview_timestamp_value(key, value);
    }

    match value {
        serde_json::Value::String(text) => compact_run_preview_text(text),
        serde_json::Value::Number(number) => Some(number.to_string()),
        serde_json::Value::Bool(flag) => Some(flag.to_string()),
        serde_json::Value::Array(items) => summarize_run_preview_array(items),
        serde_json::Value::Object(map) => summarize_run_preview_object(map),
        serde_json::Value::Null => None,
    }
}

fn is_duration_run_preview_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    lower.contains("elapsed") || lower.contains("duration")
}

fn is_time_run_preview_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    lower == "created_at"
        || lower == "started_at"
        || lower == "finished_at"
        || lower == "created_at_ms"
        || lower == "started_at_ms"
        || lower == "finished_at_ms"
}

fn format_run_preview_duration(key: &str, value: &serde_json::Value) -> Option<String> {
    let raw = json_number_as_f64(value)?;
    if !raw.is_finite() || raw < 0.0 {
        return None;
    }

    let lower = key.to_ascii_lowercase();
    let millis =
        if lower.ends_with("_ms") || lower.contains("millis") { raw } else { raw * 1000.0 };
    if millis < 1000.0 {
        return Some(format!("{} 毫秒", millis.round() as u64));
    }

    Some(format!("{} 秒", trim_decimal(millis / 1000.0)))
}

fn format_run_preview_timestamp_value(key: &str, value: &serde_json::Value) -> Option<String> {
    let raw = json_number_as_f64(value)?;
    if !raw.is_finite() || raw < 0.0 {
        return None;
    }

    let lower = key.to_ascii_lowercase();
    let millis = if lower.ends_with("_ms") || raw >= 10_000_000_000.0 { raw } else { raw * 1000.0 };
    let millis = millis.round();
    if millis > i64::MAX as f64 {
        return None;
    }

    Local
        .timestamp_millis_opt(millis as i64)
        .single()
        .map(|time| time.format("%Y/%m/%d %H:%M:%S").to_string())
}

fn json_number_as_f64(value: &serde_json::Value) -> Option<f64> {
    match value {
        serde_json::Value::Number(number) => number.as_f64(),
        serde_json::Value::String(text) => text.trim().parse::<f64>().ok(),
        _ => None,
    }
}

fn summarize_run_preview_object(
    map: &serde_json::Map<String, serde_json::Value>,
) -> Option<String> {
    if map.is_empty() {
        return None;
    }

    for key in ["text", "answer", "result", "output", "data"] {
        if let Some(value) = map.get(key).and_then(|value| workflow_run_preview_value(key, value)) {
            return Some(value);
        }
    }

    let mut parts = Vec::new();
    for (key, value) in map {
        if parts.len() >= 3 {
            break;
        }
        if is_sensitive_run_preview_key(key) {
            continue;
        }
        if let Some(value) = workflow_run_preview_value(key, value) {
            parts.push(format!("{key}={value}"));
        }
    }

    if parts.is_empty() {
        None
    } else if map.len() > parts.len() {
        Some(format!("{} 等", parts.join(", ")))
    } else {
        Some(parts.join(", "))
    }
}

fn summarize_run_preview_array(items: &[serde_json::Value]) -> Option<String> {
    if items.is_empty() {
        return None;
    }

    let mut parts = Vec::new();
    for value in items.iter().take(3) {
        let Some(value) = workflow_run_preview_value("item", value) else {
            continue;
        };
        parts.push(value);
    }

    if parts.is_empty() {
        Some(format!("{} 项", items.len()))
    } else if items.len() > parts.len() {
        Some(format!("{} 等，共 {} 项", parts.join(", "), items.len()))
    } else {
        Some(parts.join(", "))
    }
}

fn compact_run_preview_text(text: &str) -> Option<String> {
    let compact = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let compact = compact.trim();
    if compact.is_empty() {
        return None;
    }

    if compact.chars().count() <= RUN_PREVIEW_NODE_MAX_VALUE_CHARS {
        Some(compact.to_string())
    } else {
        let mut preview =
            compact.chars().take(RUN_PREVIEW_NODE_MAX_VALUE_CHARS).collect::<String>();
        preview.push_str("...");
        Some(preview)
    }
}

fn trim_decimal(value: f64) -> String {
    let mut formatted = format!("{value:.2}");
    while formatted.contains('.') && formatted.ends_with('0') {
        formatted.pop();
    }
    if formatted.ends_with('.') {
        formatted.pop();
    }
    formatted
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WorkflowPreviewStatusKind {
    Running,
    Succeeded,
    Failed,
    Canceled,
    Waiting,
    Unknown,
}

pub(super) fn workflow_run_status_label(status: WorkflowRunStatus) -> &'static str {
    match status {
        WorkflowRunStatus::Running => "运行中",
        WorkflowRunStatus::Succeeded => "成功",
        WorkflowRunStatus::Failed => "失败",
        WorkflowRunStatus::Canceled => "已取消",
    }
}

pub(super) fn workflow_run_node_status_label(status: Option<&str>) -> &'static str {
    match status {
        Some("running") | Some("started") => "运行中",
        Some("succeeded") | Some("success") | Some("finished") => "成功",
        Some("failed") | Some("error") => "失败",
        Some("canceled") | Some("cancelled") => "已取消",
        Some(_) => "未知",
        None => "等待中",
    }
}

fn workflow_run_node_status_kind(status: Option<&str>) -> WorkflowPreviewStatusKind {
    match status {
        Some("running") | Some("started") => WorkflowPreviewStatusKind::Running,
        Some("succeeded") | Some("success") | Some("finished") => {
            WorkflowPreviewStatusKind::Succeeded
        }
        Some("failed") | Some("error") => WorkflowPreviewStatusKind::Failed,
        Some("canceled") | Some("cancelled") => WorkflowPreviewStatusKind::Canceled,
        Some(_) => WorkflowPreviewStatusKind::Unknown,
        None => WorkflowPreviewStatusKind::Waiting,
    }
}

pub(super) fn workflow_run_preview_card_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    let is_dark = is_dark_theme(theme);

    iced::widget::container::Style {
        text_color: Some(theme.palette().text),
        background: Some(Background::Color(if is_dark {
            palette.background.base.color.scale_alpha(0.98)
        } else {
            Color::WHITE
        })),
        border: Border {
            width: 1.0,
            color: palette.background.strong.color.scale_alpha(if is_dark { 0.56 } else { 0.18 }),
            radius: 8.0.into(),
        },
        shadow: Shadow {
            color: Color::BLACK.scale_alpha(if is_dark { 0.34 } else { 0.16 }),
            offset: Vector::new(0.0, 12.0),
            blur_radius: 28.0,
        },
        ..Default::default()
    }
}

pub(super) fn workflow_run_preview_node_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    iced::widget::container::Style {
        background: Some(Background::Color(
            palette.background.weak.color.scale_alpha(if is_dark_theme(theme) {
                0.70
            } else {
                0.46
            }),
        )),
        border: Border {
            width: 1.0,
            color: palette.background.strong.color.scale_alpha(if is_dark_theme(theme) {
                0.42
            } else {
                0.16
            }),
            radius: 6.0.into(),
        },
        ..Default::default()
    }
}

pub(super) fn workflow_run_preview_output_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    iced::widget::container::Style {
        text_color: Some(theme.palette().text),
        background: Some(Background::Color(
            palette.background.weak.color.scale_alpha(if is_dark_theme(theme) {
                0.58
            } else {
                0.34
            }),
        )),
        border: Border {
            width: 1.0,
            color: palette.background.strong.color.scale_alpha(if is_dark_theme(theme) {
                0.34
            } else {
                0.14
            }),
            radius: 6.0.into(),
        },
        ..Default::default()
    }
}

pub(super) fn workflow_run_preview_output_empty_style(
    theme: &Theme,
) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    let is_dark = is_dark_theme(theme);
    // The empty state stays lighter than a populated report: a dashed-feel low-contrast
    // surface so "no output" reads as placeholder rather than real content.
    iced::widget::container::Style {
        background: Some(Background::Color(
            palette.background.weak.color.scale_alpha(if is_dark { 0.32 } else { 0.20 }),
        )),
        border: Border {
            width: 1.0,
            color: palette.background.strong.color.scale_alpha(if is_dark { 0.24 } else { 0.10 }),
            radius: 6.0.into(),
        },
        ..Default::default()
    }
}

fn workflow_run_preview_error_heading_style(theme: &Theme) -> iced::widget::text::Style {
    iced::widget::text::Style { color: Some(theme.extended_palette().danger.base.color) }
}

pub(super) fn workflow_run_preview_error_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    let is_dark = is_dark_theme(theme);
    // Keep the body text on the base palette text so the multi-line debug report stays
    // legible against a translucent danger tint in both themes; the danger accent is
    // reserved for the heading and border so the surface still reads as an error panel.
    iced::widget::container::Style {
        text_color: Some(theme.palette().text),
        background: Some(Background::Color(palette.danger.weak.color.scale_alpha(if is_dark {
            0.30
        } else {
            0.42
        }))),
        border: Border {
            width: 1.0,
            color: palette.danger.base.color.scale_alpha(if is_dark { 0.52 } else { 0.42 }),
            radius: 6.0.into(),
        },
        ..Default::default()
    }
}

pub(super) fn workflow_run_preview_chip_style(
    theme: &Theme,
    status: WorkflowPreviewStatusKind,
) -> iced::widget::container::Style {
    let is_dark = is_dark_theme(theme);
    let color = workflow_run_status_kind_color(theme, status);
    iced::widget::container::Style {
        background: Some(Background::Color(color.scale_alpha(if is_dark { 0.20 } else { 0.10 }))),
        border: Border {
            width: 1.0,
            color: color.scale_alpha(if is_dark { 0.48 } else { 0.28 }),
            radius: 4.0.into(),
        },
        // Keep chip text fully opaque so muted-accent kinds stay readable on dark backgrounds.
        text_color: Some(opaque_color(color)),
        ..Default::default()
    }
}

/// Drops a color's translucency so it renders at full strength as chip/label text.
fn opaque_color(color: Color) -> Color {
    Color { a: 1.0, ..color }
}

fn workflow_run_status_badge_style(
    theme: &Theme,
    status: WorkflowRunStatus,
) -> iced::widget::container::Style {
    let color = workflow_run_status_color(theme, status);

    iced::widget::container::Style {
        text_color: Some(color),
        background: Some(Background::Color(color.scale_alpha(if is_dark_theme(theme) {
            0.16
        } else {
            0.10
        }))),
        border: Border { width: 1.0, color: color.scale_alpha(0.34), radius: 4.0.into() },
        ..Default::default()
    }
}

fn workflow_status_dot<'a>(color_fn: impl Fn(&Theme) -> Color + 'a) -> Element<'a, Message> {
    container(Space::new().width(8).height(8))
        .style(move |theme: &Theme| iced::widget::container::Style {
            background: Some(Background::Color(color_fn(theme))),
            border: Border { radius: 999.0.into(), ..Default::default() },
            ..Default::default()
        })
        .into()
}

fn workflow_run_status_color(theme: &Theme, status: WorkflowRunStatus) -> Color {
    match status {
        WorkflowRunStatus::Running => theme.palette().primary,
        WorkflowRunStatus::Succeeded => theme.extended_palette().success.base.color,
        WorkflowRunStatus::Failed => theme.extended_palette().danger.base.color,
        WorkflowRunStatus::Canceled => theme.extended_palette().secondary.base.color,
    }
}

fn workflow_run_status_kind_color(theme: &Theme, status: WorkflowPreviewStatusKind) -> Color {
    let palette = theme.extended_palette();
    match status {
        WorkflowPreviewStatusKind::Running => theme.palette().primary,
        WorkflowPreviewStatusKind::Succeeded => palette.success.base.color,
        WorkflowPreviewStatusKind::Failed => palette.danger.base.color,
        WorkflowPreviewStatusKind::Canceled => palette.secondary.base.color,
        WorkflowPreviewStatusKind::Waiting => theme.palette().text.scale_alpha(0.56),
        WorkflowPreviewStatusKind::Unknown => palette.warning.base.color,
    }
}

#[cfg(test)]
#[path = "run_preview_tests.rs"]
mod run_preview_tests;
