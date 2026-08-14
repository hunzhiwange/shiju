//! 思考块视图组件。

use iced::widget::svg::{self};
use iced::widget::{column, container, mouse_area, row, text, text_editor};
use iced::{Alignment, Border, Color, Element, Length, Theme};
use std::collections::HashSet;

use crate::app::assets::Icon;
use crate::app::components::text_editor_scroll_panel::pass_wheel_to_parent;
use crate::app::models;
use crate::app::{App, Message, message};

use super::super::selectable_text::{SelectableTextPolicy, selectable_text_policy};
use super::super::tool_text_support::{
    READ_ONLY_EDITOR_MAX_HEIGHT, chat_text_font, chat_text_line_height, read_only_text_style,
};
use super::super::utils::{
    chat_secondary_muted_text_color, chat_secondary_subtle_text_color, icon_svg,
    normalize_display_text,
};
use super::styles::{
    THINK_BLOCK_CONTENT_SPACING, THINK_META_TEXT_SIZE, THINK_STATUS_TEXT_SIZE,
    think_block_text_color, thinking_status_text,
};
use super::text::{should_segment_text_block, think_text_body};

/// 渲染思考块视图
///
/// 创建一个可展开/折叠的思考块 UI 组件，显示 AI 的推理过程。
/// 包含状态标签（思考中/思考）、持续时间、展开/折叠图标和内容区域。
///
/// # 参数
/// - `app`: 应用状态引用，用于获取运行时配置和 UI 状态
/// - `msg_idx`: 消息在聊天列表中的索引
/// - `think_idx`: 思考块在当前消息中的索引
/// - `content`: 思考内容文本
/// - `open`: 思考块是否为未闭合状态（流式输出中）
/// - `timing`: 可选的思考时间信息，包含开始和结束时间戳
///
/// # 返回值
/// 返回思考块的完整 UI 元素
///
/// # 交互功能
/// - 点击头部可展开/折叠内容
/// - 鼠标悬停显示高亮效果
/// - 流式输出时实时更新状态
pub(super) fn think_block_view<'a>(
    app: &'a App,
    msg_idx: usize,
    think_idx: usize,
    content: String,
    open: bool,
    timing: Option<&'a models::ThinkTiming>,
) -> Element<'a, Message> {
    let runtime = app.current_session_runtime();
    let key = ((msg_idx as u64) << 32) | (think_idx as u64);
    let is_thinking = think_block_is_running(open, timing);
    let default_expanded =
        think_block_default_expanded(app.dialogue_flow_show_reasoning_summary, open, timing);
    let expanded = think_block_resolved_expanded(
        default_expanded,
        key,
        &app.chat_think_expanded,
        &app.chat_think_collapsed,
    );
    let is_streaming_msg = runtime.is_requesting && msg_idx + 1 == app.chat.len();
    let now_ms = crate::app::time::now_ms();

    let (status_label, duration_label) = if let Some(timing) = timing {
        let end_ms = if is_thinking { now_ms } else { timing.end_ms.unwrap_or(now_ms) };
        let duration_secs = ((end_ms.saturating_sub(timing.start_ms)) as f64 / 1000.0) as u64;
        let status = if is_thinking { "思考中" } else { "思考" };
        (status.to_string(), Some(format!("{} 秒", duration_secs)))
    } else if is_thinking {
        ("思考中".to_string(), None)
    } else {
        ("思考".to_string(), None)
    };

    let toggle_icon = if expanded { Icon::ChevronDown } else { Icon::ChevronRight };
    let is_hovered = app.chat_think_hovered_idx == Some(key);
    let toggle_icon = icon_svg(toggle_icon)
        .width(Length::Fixed(10.0))
        .height(Length::Fixed(10.0))
        .style(move |theme: &Theme, _status| {
            let fg = if is_hovered {
                chat_secondary_muted_text_color(theme)
            } else {
                chat_secondary_subtle_text_color(theme)
            };
            svg::Style { color: Some(fg) }
        });

    let status_text: Element<'_, Message> = if is_thinking {
        thinking_status_text(&status_label, now_ms, app.status_animation_frame)
    } else {
        text(status_label)
            .size(THINK_STATUS_TEXT_SIZE)
            .font(chat_text_font())
            .style(move |theme: &Theme| iced::widget::text::Style {
                color: Some(chat_secondary_subtle_text_color(theme)),
            })
            .into()
    };

    let mut head_row = row![status_text].spacing(5).align_y(Alignment::Center);

    if let Some(duration) = duration_label {
        head_row = head_row.push(
            text(format!("· {duration}")).size(THINK_META_TEXT_SIZE).font(chat_text_font()).style(
                |theme: &Theme| iced::widget::text::Style {
                    color: Some(chat_secondary_subtle_text_color(theme)),
                },
            ),
        );
    }

    head_row = head_row.push(container(toggle_icon).padding([1, 2]));

    let head = mouse_area(container(head_row).width(Length::Fill).padding([1, 0]))
        .on_enter(Message::Chat(message::ChatMessage::ThinkHover(msg_idx, think_idx)))
        .on_exit(Message::Chat(message::ChatMessage::ThinkHoverLeave));

    let head = head.on_press(Message::Chat(message::ChatMessage::ToggleThink(
        msg_idx,
        think_idx,
        default_expanded,
    )));

    let mut think_col = column![head].spacing(THINK_BLOCK_CONTENT_SPACING);

    if expanded {
        let text_content = normalize_display_text(content.trim()).into_owned();
        if !text_content.is_empty() {
            let scroll_key = ((msg_idx as u64) << 32) | (think_idx as u64);
            let use_segmented_body = should_segment_text_block(&text_content);
            let prefer_plain_text =
                super::text::should_prefer_plain_think_body(is_streaming_msg, is_thinking);

            if use_segmented_body || prefer_plain_text {
                think_col = think_col.push(think_text_body(
                    app,
                    msg_idx,
                    think_idx,
                    text_content.clone(),
                    prefer_plain_text,
                ));
            } else if !is_streaming_msg
                && selectable_text_policy(&text_content, false)
                    != SelectableTextPolicy::PreviewWithCopyFallback
            {
                if let Some(editor_content) = app.chat_think_editors.get(&scroll_key) {
                    let on_action = move |action| {
                        Message::Chat(message::ChatMessage::ThinkEditorAction(
                            msg_idx, think_idx, action,
                        ))
                    };

                    let editor = text_editor(editor_content)
                        .on_action(on_action)
                        .size(14.0)
                        .line_height(chat_text_line_height())
                        .padding(0)
                        .height(Length::Shrink)
                        .max_height(READ_ONLY_EDITOR_MAX_HEIGHT)
                        .font(chat_text_font())
                        .style(move |theme: &Theme, _status: text_editor::Status| {
                            let value = think_block_text_color(theme);
                            read_only_text_style(theme, value)
                        });

                    let body = container(pass_wheel_to_parent(editor)).width(Length::Fill).style(
                        |_theme: &Theme| iced::widget::container::Style {
                            background: None,
                            border: Border {
                                width: 0.0,
                                color: Color::TRANSPARENT,
                                radius: 10.0.into(),
                            },
                            ..Default::default()
                        },
                    );
                    think_col = think_col.push(body);
                } else {
                    let body = container(
                        text(text_content.clone())
                            .size(14)
                            .font(chat_text_font())
                            .line_height(chat_text_line_height())
                            .style(|theme: &Theme| iced::widget::text::Style {
                                color: Some(think_block_text_color(theme)),
                            }),
                    )
                    .width(Length::Fill)
                    .style(|_theme: &Theme| iced::widget::container::Style {
                        background: None,
                        border: Border {
                            width: 0.0,
                            color: Color::TRANSPARENT,
                            radius: 10.0.into(),
                        },
                        ..Default::default()
                    });
                    think_col = think_col.push(body);
                }
            } else {
                think_col = think_col.push(think_text_body(
                    app,
                    msg_idx,
                    think_idx,
                    text_content.clone(),
                    prefer_plain_text,
                ));
            }
        }
    }

    let think_box = container(think_col);

    think_box.width(Length::Fill).into()
}

pub(super) fn think_block_is_running(open: bool, timing: Option<&models::ThinkTiming>) -> bool {
    open || timing.is_some_and(|think_timing| think_timing.end_ms.is_none())
}

pub(crate) fn think_block_default_expanded(
    _show_reasoning_summary: bool,
    _open: bool,
    _timing: Option<&models::ThinkTiming>,
) -> bool {
    false
}

pub(crate) fn think_block_resolved_expanded(
    default_expanded: bool,
    key: u64,
    manual_expanded: &HashSet<u64>,
    manual_collapsed: &HashSet<u64>,
) -> bool {
    if manual_collapsed.contains(&key) {
        false
    } else if manual_expanded.contains(&key) {
        true
    } else {
        default_expanded
    }
}

pub(crate) fn should_render_think_block(
    show_reasoning_summary: bool,
    total_think_blocks: usize,
    open: bool,
    timing: Option<&models::ThinkTiming>,
) -> bool {
    let _ = total_think_blocks;
    think_block_is_running(open, timing) || show_reasoning_summary
}
