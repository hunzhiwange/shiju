//! Workflow saved app run preview overlay.

use super::*;
use desktop_ui_controls::system_settings_common::settings_modal_card;
use chrono::{Local, TimeZone};
use iced::widget::{column, row};
use state::{WorkflowRunNodePreview, WorkflowRunPreview, WorkflowRunStatus};

const RUN_PREVIEW_WIDTH: f32 = 860.0;
const RUN_PREVIEW_MAX_HEIGHT: f32 = 760.0;
const RUN_PREVIEW_NODE_LABEL_WIDTH: f32 = 74.0;
const RUN_PREVIEW_NODE_RAIL_WIDTH: f32 = 12.0;
const RUN_PREVIEW_NODE_MAX_FIELDS: usize = 8;
const RUN_PREVIEW_NODE_MAX_VALUE_CHARS: usize = 96;
pub const RUN_PREVIEW_PENDING_RUN_ID_LABEL: &str = "等待运行 ID";
const RUN_PREVIEW_EMPTY_TIMELINE_LABEL: &str = "等待节点事件...";

pub fn saved_app_run_preview_close_message() -> WorkflowMessage {
    WorkflowMessage::CloseSavedAppRunPreview
}

/// Resolves the summary's run-id label, falling back to a placeholder before the
/// gateway assigns a run id so the header stays stable for a freshly started run.
pub fn run_preview_run_id_label(preview: &WorkflowRunPreview) -> &str {
    preview.run_id.as_deref().unwrap_or(RUN_PREVIEW_PENDING_RUN_ID_LABEL)
}

pub fn saved_app_run_preview_action(
    i18n: desktop_i18n::I18n,
    preview: &WorkflowRunPreview,
) -> Option<(&'static str, WorkflowMessage)> {
    match (preview.is_finished, preview.status) {
        (false, WorkflowRunStatus::Running) => {
            Some((i18n.__("取消运行"), WorkflowMessage::CancelSavedAppRun))
        }
        (true, WorkflowRunStatus::Succeeded | WorkflowRunStatus::Failed) => {
            Some((i18n.__("重新执行"), WorkflowMessage::RetrySavedAppRunPreview))
        }
        _ => None,
    }
}

pub fn build_saved_app_run_preview_layer(
    i18n: desktop_i18n::I18n,
    preview: &WorkflowRunPreview,
    error_copied: bool,
) -> Element<'_, Message> {
    let close_message = saved_app_run_preview_close_message();
    let run_id_label = run_preview_run_id_label(preview);
    let preview_action = saved_app_run_preview_action(i18n, preview);
    let summary = row![
        workflow_run_status_badge(i18n, preview.status),
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
            timeline = timeline.push(workflow_run_node_row(i18n, index, node));
        }
    }

    let output_action = workflow_run_output_text(preview.text_output.as_str()).map(|_| {
        run_preview_detail_button(
            i18n,
            WorkflowMessage::OpenSavedAppRunOutputDetail,
        )
    });

    let mut body = column![
        run_preview_section(i18n.__("节点时间线"), None, timeline.into()),
        run_preview_section(
            i18n.__("输出"),
            output_action,
            workflow_run_output(i18n, preview.text_output.as_str()),
        ),
    ]
    .spacing(16)
    .width(Length::Fill);

    if preview.error.is_some() {
        body = body.push(run_preview_section(
            i18n.__("错误详情"),
            Some(run_preview_copy_button(
                i18n,
                error_copied,
                WorkflowMessage::CopySavedAppRunError,
            )),
            workflow_run_error(i18n, preview),
        ));
    }

    let heading = settings_modal_header(
        column![
            text(i18n.__("运行预览")).size(20),
            text(i18n.__("实时展示 Workflow 执行过程")).size(13).style(settings_muted_text_style),
        ]
        .spacing(4),
        settings_close_button(close_message.clone()),
    );
    let header = column![heading, summary].spacing(12);
    let footer = preview_action.map(|(label, action)| {
        let button_style = match action {
            WorkflowMessage::CancelSavedAppRun => danger_action_btn_style,
            WorkflowMessage::RetrySavedAppRunPreview => primary_action_btn_style,
            _ => rounded_action_btn_style,
        };
        row![
            Space::new().width(Length::Fill),
            button(text(label).size(13))
                .style(button_style)
                .padding([8, 14])
                .on_press(action),
        ]
        .into()
    });
    let card = settings_modal_layout(header, body, footer)
        .width(RUN_PREVIEW_WIDTH)
        .height(RUN_PREVIEW_MAX_HEIGHT);
    settings_modal_overlay(None, close_message, container(card).padding(12))
}

/// 构建运行预览的“查看详细”弹窗：以只读、可滚动、可选中复制的编辑器展示完整内容。
pub fn build_saved_app_run_detail_layer(
    i18n: desktop_i18n::I18n,
    detail: &state::WorkflowRunDetailDialog,
) -> Element<'_, Message> {
    let close_message = WorkflowMessage::CloseSavedAppRunDetail;

    let editor = text_editor(&detail.content)
        .font(iced::Font::DEFAULT)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .padding(SETTINGS_CONTROL_PADDING)
        .height(Length::Fill)
        .on_action(|action| WorkflowMessage::SavedAppRunDetailAction(action))
        .style(settings_text_editor_style);
    let header = settings_modal_header(
        column![
            text(detail.title_source.map(|source| i18n.__(source)).unwrap_or(&detail.title))
                .size(20),
            text(i18n.__("可滚动并选中复制完整内容")).size(13).style(settings_muted_text_style),
        ]
        .spacing(4),
        settings_close_button(close_message.clone()),
    );
    // The editor already scrolls internally so selection and copy keep their original behavior.
    let card = settings_modal_card(column![header, editor].spacing(16)).width(760).height(560);
    settings_modal_overlay(None, close_message, container(card).padding(12))
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
fn run_preview_detail_button(
    i18n: desktop_i18n::I18n,
    on_press: Message,
) -> Element<'static, Message> {
    button(text(i18n.__("查看详细")).size(11))
        .style(rounded_action_btn_style)
        .padding([4, 10])
        .on_press(on_press)
        .into()
}

/// 渲染错误详情的复制按钮：复制后短暂显示 ✓，2 秒后由计时消息恢复为“复制”。
fn run_preview_copy_button(
    i18n: desktop_i18n::I18n,
    copied: bool,
    on_press: Message,
) -> Element<'static, Message> {
    let label = if copied { "✓" } else { i18n.__("复制") };
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

fn workflow_run_status_badge(
    i18n: desktop_i18n::I18n,
    status: WorkflowRunStatus,
) -> Element<'static, Message> {
    let label = workflow_run_status_label(i18n, status);
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

fn workflow_run_node_row(
    i18n: desktop_i18n::I18n,
    index: usize,
    node: &WorkflowRunNodePreview,
) -> Element<'_, Message> {
    let status = workflow_run_node_status_label(i18n, node.status.as_deref());
    let status_kind = workflow_run_node_status_kind(node.status.as_deref());
    let data_preview = workflow_run_node_data_preview_items(i18n, node);

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
        run_preview_detail_button(
            i18n,
            WorkflowMessage::OpenSavedAppRunNodeDetail(index)
        ),
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
            workflow_run_node_details(i18n, data_preview),
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
pub fn workflow_run_output_text(output: &str) -> Option<String> {
    let trimmed = output.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

pub fn workflow_run_output(
    i18n: desktop_i18n::I18n,
    output: &str,
) -> Element<'_, Message> {
    match workflow_run_output_text(output) {
        None => container(text(i18n.__("暂无文本输出")).size(12).style(settings_muted_text_style))
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

pub fn workflow_run_error(
    i18n: desktop_i18n::I18n,
    preview: &WorkflowRunPreview,
) -> Element<'_, Message> {
    let report = column![
        text(i18n.__("失败详情"))
            .size(12)
            .line_height(1.0)
            .style(workflow_run_preview_error_heading_style),
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

fn workflow_run_node_details(
    i18n: desktop_i18n::I18n,
    items: Vec<(String, String)>,
) -> Element<'static, Message> {
    if items.is_empty() {
        return text(i18n.__("暂无节点详情")).size(11).style(settings_muted_text_style).into();
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

pub fn workflow_run_node_data_preview_items(
    i18n: desktop_i18n::I18n,
    node: &WorkflowRunNodePreview,
) -> Vec<(String, String)> {
    let mut items = Vec::new();
    let mut shown_keys = Vec::new();
    let Some(map) = node.data.as_object() else {
        return workflow_run_preview_value(i18n, "data", &node.data)
            .map(|value| vec![("data".to_string(), value)])
            .unwrap_or_default();
    };

    for (key, label) in [
        ("node_type", i18n.__("类型")),
        ("index", i18n.__("序号")),
        ("predecessor_node_id", i18n.__("上游")),
        ("created_at", i18n.__("开始时间")),
        ("created_at_ms", i18n.__("开始时间")),
        ("started_at", i18n.__("开始时间")),
        ("started_at_ms", i18n.__("开始时间")),
        ("finished_at", i18n.__("完成时间")),
        ("finished_at_ms", i18n.__("完成时间")),
        ("elapsed_time", i18n.__("耗时")),
        ("elapsed_ms", i18n.__("耗时")),
        ("duration_ms", i18n.__("耗时")),
        ("error", i18n.__("错误")),
        ("inputs", i18n.__("输入")),
        ("process_data", i18n.__("过程")),
        ("outputs", i18n.__("输出")),
    ] {
        shown_keys.push(key.to_string());
        if let Some(value) =
            map.get(key).and_then(|value| workflow_run_preview_value(i18n, key, value))
        {
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
        if let Some(value) = workflow_run_preview_value(i18n, key, value) {
            items.push((format!("{key}:"), value));
        }
    }

    items
}


mod formatting;
use formatting::*;
mod styles;
pub use styles::{WorkflowPreviewStatusKind, workflow_run_status_label, workflow_run_node_status_label, workflow_run_preview_node_style, workflow_run_preview_output_style, workflow_run_preview_output_empty_style, workflow_run_preview_error_style, workflow_run_preview_chip_style};
use styles::*;

#[cfg(test)]
#[path = "run_preview_tests.rs"]
mod run_preview_tests;
