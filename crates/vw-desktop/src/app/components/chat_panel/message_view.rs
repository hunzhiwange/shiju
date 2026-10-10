//! 消息视图渲染模块
//!
//! 本模块负责将聊天消息渲染为 Iced UI 元素。主要功能包括：
//! - 解析消息内容中的特殊区块（思考块、工具调用块、文本块）
//! - 渲染不同角色（用户、助手、系统）的消息气泡
//! - 处理消息的交互功能（复制、展开/折叠思考块、编辑器集成）
//!
//! ## 主要组件
//!
//! - [`message_view`]: 消息视图的主入口函数，负责完整消息气泡的渲染
//! - [`think_block_view`]: 渲染可展开/折叠的思考块
//! - [`assistant_api_error_view`]: 渲染 API 错误信息的特殊视图
//!
//! ## 内容解析
//!
//! 消息内容可能包含以下特殊区块：
//! - `<think ...>...</think`>: 思考块，显示 AI 的推理过程
//! - `tool ...`: 工具调用块，记录 AI 执行的工具操作

mod assistant_body;
mod assistant_error;
mod markdown;
mod parse;
mod render_cache;
mod styles;
mod text;
mod think_block;
mod tool_summaries;

#[cfg(test)]
mod assistant_body_tests;
#[cfg(test)]
mod assistant_error_tests;
#[cfg(test)]
mod markdown_tests;
#[cfg(test)]
mod message_view_tests;
#[cfg(test)]
mod parse_tests;
#[cfg(test)]
mod render_cache_tests;
#[cfg(test)]
mod styles_tests;
#[cfg(test)]
mod text_tests;
#[cfg(test)]
mod think_block_tests;
#[cfg(test)]
mod tool_summaries_tests;

use iced::widget::svg::{self};
use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{Space, button, column, container, row, text as iced_text};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};
use std::borrow::Cow;
use vw_shared::session::ui_types::ProcessingTiming;

use crate::app::assets::Icon;
use crate::app::components::input_panel::attachment::{
    attachment_preview_strip, parse_attachment_markers,
};
use crate::app::components::widgets::RightClickArea;
use crate::app::models::{self, ChatRole};
use crate::app::{App, Message, message};

use self::assistant_body::render_assistant_body;
use self::render_cache::resolve_visible_text_and_copy_hash;
use self::styles::{
    ASSISTANT_HEADER_BODY_SPACING, MESSAGE_META_TEXT_SIZE, PROCESSING_DURATION_TEXT_SIZE,
    is_dark_theme, message_action_button_style, message_action_icon_color, message_meta_text_color,
    processing_duration_text_color,
};
use self::text::{
    message_text_body, session_control_selection_card, split_session_control_selection,
    task_session_visible_text, user_message_editor_body,
};

use super::selectable_text::{SelectableTextPolicy, selectable_text_policy};
use super::tool_text_support::{chat_text_font, selected_chat_text_for_message};
use super::tools::{
    pending_permission_badge_label, pending_permission_request_badge_label,
    pending_permission_request_for_message, pending_permission_targets_message,
};
use super::utils::{
    chat_context_target_key, copy_tooltip_content, icon_svg, is_recent_copy, normalize_display_text,
};

pub(crate) use self::assistant_body::deduped_tool_last_indices;
#[cfg(test)]
pub(crate) use self::assistant_body::should_highlight_pending_permission_tool;
pub(crate) use self::markdown::{MarkdownBlock, markdown_block_key, parse_markdown_blocks};
pub(crate) use self::parse::hash_chat_content;
pub(crate) use self::render_cache::{
    assistant_render_blocks, build_render_cache_entry, effective_assistant_render_cache,
};
pub(crate) use self::styles::{message_surface_contract, message_surface_style};
pub(crate) use self::text::estimate_message_height_rough;
#[cfg(test)]
pub(crate) use self::text::should_prefer_plain_think_body;
#[cfg(test)]
pub(crate) use self::think_block::{
    should_render_think_block, think_block_default_expanded, think_block_resolved_expanded,
};
#[cfg(test)]
pub(crate) use self::tool_summaries::explore_summary_text_blocks;
#[cfg(test)]
pub(crate) use self::tool_summaries::summarize_explore_items;
#[cfg(test)]
pub(crate) use self::tool_summaries::tool_card_text_blocks;
#[cfg(test)]
pub(crate) use self::tool_summaries::trailing_tool_tail_text_source_block_idx;

pub(crate) fn message_footer_padding(is_user: bool) -> iced::Padding {
    if is_user { [0, 6].into() } else { iced::Padding::ZERO }
}

fn processing_duration_label(
    timing: Option<&ProcessingTiming>,
    live_elapsed_ms: Option<u64>,
) -> Option<String> {
    let timing = timing?;
    let elapsed_ms = if let Some(completed_ms) = timing.completed_ms {
        timing
            .processing_elapsed_ms
            .unwrap_or_else(|| completed_ms.saturating_sub(timing.started_ms))
    } else {
        live_elapsed_ms?
    };
    let total_seconds = elapsed_ms / 1_000;
    let hours = total_seconds / 3_600;
    let minutes = total_seconds % 3_600 / 60;
    let seconds = total_seconds % 60;
    let duration = if hours > 0 {
        format!("{hours}h {minutes}m {seconds}s")
    } else if minutes > 0 {
        format!("{minutes}m {seconds}s")
    } else {
        format!("{seconds}s")
    };
    Some(format!("已处理 {duration}"))
}

/// 渲染消息视图（主入口函数）
///
/// 创建完整的消息气泡 UI，包括消息内容、元信息（模型名称、时间）和操作按钮（复制）。
/// 根据消息角色（用户/助手/系统）采用不同的布局和样式。
pub fn message_view<'a>(
    app: &'a App,
    idx: usize,
    role: ChatRole,
    content: &'a str,
    think_timing: &'a [models::ThinkTiming],
    processing_timing: Option<&ProcessingTiming>,
    message_meta: Option<Cow<'a, str>>,
    render_cache: &'a models::ChatRenderCacheEntry,
    _enable_heavy_tool_content: bool,
) -> Element<'a, Message> {
    let task_session_content = if role == ChatRole::User {
        task_session_visible_text(content)
    } else {
        Cow::Borrowed(content)
    };
    let content = task_session_content.as_ref();
    let runtime = app.current_session_runtime();
    let is_user = role == ChatRole::User;
    let assistant_like = matches!(role, ChatRole::Assistant | ChatRole::Tool);
    let is_streaming_assistant =
        role == ChatRole::Assistant && runtime.is_requesting && idx + 1 == app.chat.len();
    let live_processing_elapsed_ms = is_streaming_assistant
        .then(|| runtime.processing_timer.elapsed_ms_at(web_time::Instant::now()));
    let content_hash = hash_chat_content(content);
    let (visible_for_copy, visible_content_hash) =
        resolve_visible_text_and_copy_hash(app, idx, content, content_hash, render_cache);
    let render_cache = effective_assistant_render_cache(
        content,
        content_hash,
        render_cache,
        visible_for_copy.as_ref(),
        visible_content_hash,
        is_streaming_assistant,
        app.dialogue_flow_show_reasoning_summary,
    );
    let (assistant_blocks, has_special_assistant_blocks) =
        assistant_render_blocks(content, content_hash, render_cache.as_ref(), false);
    let assistant_has_tool_blocks =
        assistant_blocks.iter().any(|block| matches!(block, models::ParsedChatBlock::Tool { .. }));
    let (cleaned_attachment_text, attachment_items) = parse_attachment_markers(content);
    let prefer_attachment_card_render =
        !attachment_items.is_empty() && !has_special_assistant_blocks;

    let recently_copied = is_recent_copy(app, visible_content_hash);
    let copy_icon = if recently_copied { Icon::Check } else { Icon::Copy };
    let copy_tip = if recently_copied { "已复制" } else { "复制消息" };
    let permission_badge_text = pending_permission_badge_label(
        &app.permission_modal_requests,
        app.permission_modal_request_id.as_deref(),
    );
    let message_id = app.chat_message_ids.get(idx).and_then(|message_id| message_id.as_deref());
    let supports_permission_badge =
        role == ChatRole::Tool || (role == ChatRole::Assistant && assistant_has_tool_blocks);
    let is_permission_target =
        pending_permission_targets_message(app.permission_modal_request.as_ref(), message_id)
            && supports_permission_badge;
    let matched_permission_request = supports_permission_badge
        .then(|| pending_permission_request_for_message(&app.permission_modal_requests, message_id))
        .flatten();
    let permission_badge: Option<(String, Option<Message>)> = if is_permission_target {
        Some((
            permission_badge_text.clone(),
            app.permission_modal_request_id.clone().map(|request_id| {
                Message::Chat(message::ChatMessage::PermissionSelectRequest(request_id))
            }),
        ))
    } else {
        matched_permission_request.map(|request| {
            (
                pending_permission_request_badge_label(
                    &app.permission_modal_requests,
                    request.id.as_str(),
                ),
                Some(Message::Chat(message::ChatMessage::PermissionSelectRequest(
                    request.id.clone(),
                ))),
            )
        })
    };
    let can_branch_or_reset = is_user && app.active_session_id.is_some();

    let icon_action_btn = |icon: Icon,
                           label: &'static str,
                           on_press: Message,
                           highlighted: bool,
                           compact_square: bool|
     -> Element<'a, Message> {
        // 紧凑动作图标（复制、分叉、重置）与相邻元信息文字同高，避免图标显得突兀偏大。
        let icon_size = if compact_square { MESSAGE_META_TEXT_SIZE } else { 12.0 };
        let compact_button_size = MESSAGE_META_TEXT_SIZE;
        let btn = button(
            icon_svg(icon).width(Length::Fixed(icon_size)).height(Length::Fixed(icon_size)).style(
                move |theme: &Theme, status| svg::Style {
                    color: Some(message_action_icon_color(
                        theme,
                        status,
                        highlighted,
                        compact_square,
                        is_user,
                    )),
                },
            ),
        )
        .width(if compact_square {
            Length::Fixed(compact_button_size)
        } else {
            Length::Fixed(24.0)
        })
        .height(if compact_square {
            Length::Fixed(compact_button_size)
        } else {
            Length::Fixed(22.0)
        })
        .padding(0)
        .style(move |theme: &Theme, status| {
            message_action_button_style(theme, status, compact_square)
        })
        .on_press(on_press);

        Tooltip::new(btn, copy_tooltip_content(label), TooltipPosition::Top).gap(6).into()
    };

    let approval_badge_view = |label: String, on_press: Option<Message>| -> Element<'a, Message> {
        let badge =
            container(iced_text(label).size(11).font(chat_text_font()).style(|theme: &Theme| {
                iced::widget::text::Style {
                    color: Some(if is_dark_theme(theme) {
                        Color::from_rgba8(255, 235, 177, 0.96)
                    } else {
                        Color::from_rgba8(142, 103, 16, 1.0)
                    }),
                }
            }))
            .padding([2, 8])
            .style(|theme: &Theme| {
                let is_dark = is_dark_theme(theme);
                iced::widget::container::Style {
                    background: Some(Background::Color(if is_dark {
                        Color::from_rgba8(79, 62, 21, 0.72)
                    } else {
                        Color::from_rgba8(255, 245, 213, 1.0)
                    })),
                    border: Border {
                        width: 1.0,
                        color: if is_dark {
                            Color::from_rgba8(168, 132, 45, 0.72)
                        } else {
                            Color::from_rgba8(222, 187, 102, 0.92)
                        },
                        radius: 999.0.into(),
                    },
                    ..Default::default()
                }
            });

        if let Some(on_press) = on_press {
            button(badge)
                .padding(0)
                .style(|_theme: &Theme, _status| iced::widget::button::Style {
                    background: None,
                    border: Border::default(),
                    shadow: iced::Shadow::default(),
                    ..Default::default()
                })
                .on_press(on_press)
                .into()
        } else {
            badge.into()
        }
    };

    let copy_message = Message::CopyCode(visible_for_copy.to_string());

    let fork_press = Message::Chat(message::ChatMessage::OpenForkSessionDialog(idx));
    let reset_press = Message::Chat(message::ChatMessage::ToggleResetMenu(idx));

    let body = if assistant_like && !prefer_attachment_card_render {
        render_assistant_body(
            app,
            idx,
            think_timing,
            &render_cache.special_text_blocks,
            &render_cache.display_text,
            assistant_blocks.as_ref(),
            &render_cache.tool_block_metadata,
            has_special_assistant_blocks,
            is_streaming_assistant,
            visible_for_copy.as_ref(),
        )
    } else {
        let mut body = column![].spacing(8);
        let use_editor =
            should_use_user_message_editor(app, idx, content, attachment_items.is_empty());
        let (text, selection) = split_session_control_selection(cleaned_attachment_text.trim());
        if use_editor {
            if let Some(view) = user_message_editor_body(app, idx, app.theme().palette().text) {
                body = body.push(view);
            }
        } else {
            let text = normalize_display_text(text.trim()).into_owned();
            if !text.is_empty() {
                body = body.push(message_text_body(text, true));
            }
        }
        if let Some(selection) = selection {
            body = body.push(session_control_selection_card(selection));
        }
        body
    };
    let body = if prefer_attachment_card_render {
        let mut attachment_body = column![].spacing(8);
        if !attachment_items.is_empty() {
            attachment_body =
                attachment_body.push(attachment_preview_strip(attachment_items.clone()));
        }
        let (text, selection) = split_session_control_selection(cleaned_attachment_text.trim());
        let text = normalize_display_text(text.trim()).into_owned();
        if !text.is_empty() {
            attachment_body = attachment_body.push(message_text_body(text, true));
        }
        if let Some(selection) = selection {
            attachment_body = attachment_body.push(session_control_selection_card(selection));
        }
        attachment_body
    } else if !attachment_items.is_empty() && !assistant_like {
        let mut attachment_body =
            column![attachment_preview_strip(attachment_items.clone())].spacing(8);
        let (text, selection) = split_session_control_selection(cleaned_attachment_text.trim());
        let text = normalize_display_text(text.trim()).into_owned();
        if !text.is_empty() {
            attachment_body = attachment_body.push(message_text_body(text, true));
        }
        if let Some(selection) = selection {
            attachment_body = attachment_body.push(session_control_selection_card(selection));
        }
        attachment_body
    } else {
        body
    };
    let body = if assistant_like {
        let mut assistant_content = column![].spacing(ASSISTANT_HEADER_BODY_SPACING);
        if let Some(label) =
            processing_duration_label(processing_timing, live_processing_elapsed_ms)
        {
            assistant_content = assistant_content.push(
                iced_text(label).size(PROCESSING_DURATION_TEXT_SIZE).style(|theme: &Theme| {
                    iced::widget::text::Style { color: Some(processing_duration_text_color(theme)) }
                }),
            );
        }
        assistant_content.push(body)
    } else {
        body
    };

    let context_key = chat_context_target_key(idx, None);
    let surface_contract = message_surface_contract(is_user);
    let bubble_inner = container(body)
        .padding(surface_contract.padding)
        .width(surface_contract.width)
        .style(move |theme: &Theme| message_surface_style(theme, is_user, is_permission_target));
    let bubble_inner = if let Some(max_width) = surface_contract.max_width {
        bubble_inner.max_width(max_width)
    } else {
        bubble_inner
    };

    let context_text = selected_chat_text_for_message(app, idx).unwrap_or_else(|| {
        if !visible_for_copy.trim().is_empty() {
            visible_for_copy.to_string()
        } else {
            content.to_string()
        }
    });

    let bubble_inner: Element<'a, Message> = RightClickArea::new(
        bubble_inner.into(),
        Box::new(move |point| {
            Message::Chat(message::ChatMessage::OpenMessageContextMenu {
                target: context_key,
                x: point.x,
                y: point.y,
                text: context_text.clone(),
            })
        }),
    )
    .use_global_position()
    .preserve_on_right_click()
    .into();

    let footer_element: Element<'a, Message> = {
        let mut footer_col = column![].spacing(6);

        let meta_row: Element<'a, Message> = if let Some(meta_text) = message_meta {
            let meta_view = iced_text(meta_text)
                .size(MESSAGE_META_TEXT_SIZE)
                .font(chat_text_font())
                .style(move |theme: &Theme| iced::widget::text::Style {
                    color: Some(message_meta_text_color(theme, is_user)),
                });
            let approval_badge: Option<Element<'a, Message>> = permission_badge
                .clone()
                .map(|(label, on_press)| approval_badge_view(label, on_press));

            if is_user {
                let mut row = row![container(Space::new()).width(Length::Fill), meta_view]
                    .spacing(8)
                    .align_y(Alignment::Center);
                if let Some(approval_badge) = approval_badge {
                    row = row.push(approval_badge);
                }
                if can_branch_or_reset {
                    row = row.push(icon_action_btn(
                        Icon::GitBranch,
                        "分叉到新会话",
                        fork_press.clone(),
                        false,
                        true,
                    ));
                    row = row.push(icon_action_btn(
                        Icon::ArrowCounterClockwise,
                        "重置到此点",
                        reset_press.clone(),
                        false,
                        true,
                    ));
                }
                row.push(icon_action_btn(
                    copy_icon,
                    copy_tip,
                    copy_message.clone(),
                    recently_copied,
                    true,
                ))
                .into()
            } else {
                let mut row = row![meta_view].spacing(8).align_y(Alignment::Center);
                if let Some(approval_badge) = approval_badge {
                    row = row.push(approval_badge);
                }
                if can_branch_or_reset {
                    row = row.push(icon_action_btn(
                        Icon::GitBranch,
                        "分叉到新会话",
                        fork_press.clone(),
                        false,
                        true,
                    ));
                    row = row.push(icon_action_btn(
                        Icon::ArrowCounterClockwise,
                        "重置到此点",
                        reset_press.clone(),
                        false,
                        true,
                    ));
                }
                row.push(icon_action_btn(
                    copy_icon,
                    copy_tip,
                    copy_message.clone(),
                    recently_copied,
                    true,
                ))
                .into()
            }
        } else if is_user {
            let mut row = row![container(Space::new()).width(Length::Fill)]
                .spacing(8)
                .align_y(Alignment::Center);
            if let Some((label, on_press)) = permission_badge.clone() {
                row = row.push(approval_badge_view(label, on_press));
            }
            if can_branch_or_reset {
                row = row.push(icon_action_btn(
                    Icon::GitBranch,
                    "分叉到新会话",
                    fork_press.clone(),
                    false,
                    true,
                ));
                row = row.push(icon_action_btn(
                    Icon::ArrowCounterClockwise,
                    "重置到此点",
                    reset_press.clone(),
                    false,
                    true,
                ));
            }
            row.push(icon_action_btn(
                copy_icon,
                copy_tip,
                copy_message.clone(),
                recently_copied,
                true,
            ))
            .into()
        } else {
            let mut row = row![].spacing(8).align_y(Alignment::Center);
            if let Some((label, on_press)) = permission_badge.clone() {
                row = row.push(approval_badge_view(label, on_press));
            }
            if can_branch_or_reset {
                row = row.push(icon_action_btn(
                    Icon::GitBranch,
                    "分叉到新会话",
                    fork_press.clone(),
                    false,
                    true,
                ));
                row = row.push(icon_action_btn(
                    Icon::ArrowCounterClockwise,
                    "重置到此点",
                    reset_press.clone(),
                    false,
                    true,
                ));
            }
            row.push(icon_action_btn(
                copy_icon,
                copy_tip,
                copy_message.clone(),
                recently_copied,
                true,
            ))
            .into()
        };

        footer_col = footer_col.push(meta_row);
        footer_col.into()
    };

    let footer_container = container(footer_element)
        .padding(message_footer_padding(is_user))
        .width(Length::Fill)
        .align_x(if is_user {
            iced::alignment::Horizontal::Right
        } else {
            iced::alignment::Horizontal::Left
        });

    if is_user {
        column![
            container(bubble_inner).width(Length::Fill).align_x(iced::alignment::Horizontal::Right),
            row![footer_container].align_y(Alignment::Center)
        ]
        .spacing(4)
        .into()
    } else {
        column![
            container(bubble_inner).width(Length::Fill).align_x(iced::alignment::Horizontal::Left),
            row![footer_container].align_y(Alignment::Center)
        ]
        .spacing(4)
        .into()
    }
}

pub(super) fn should_use_user_message_editor(
    app: &App,
    idx: usize,
    content: &str,
    has_no_attachments: bool,
) -> bool {
    has_no_attachments
        && selectable_text_policy(content, false) != SelectableTextPolicy::PreviewWithCopyFallback
        && app.chat_message_editors.get(idx).is_some_and(|editor| !editor.text().trim().is_empty())
}
