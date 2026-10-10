//! 助手消息主体渲染。
//!
//! 该模块负责把已经解析好的助手消息块拼装为最终 UI，包含思考块、工具卡、
//! 特殊文本与探索摘要的组织顺序。

use std::collections::HashMap;

use iced::widget::{Column, Space, button, column, container, row, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};

use crate::app::models::{self, ParsedChatBlock};
use crate::app::{App, Message};

use super::super::selectable_text::{SelectableTextPolicy, selectable_text_policy};
use super::super::tool_text_support::chat_text_font_name;
use super::super::tools::{
    CommandItem, ExploreItem, ToolTextTarget, is_command_tool, is_explore_tool,
    pending_permission_badge_label, pending_permission_request_badge_label,
    pending_permission_request_for_tool_call, pending_permission_targets_message,
    pending_permission_targets_tool_call, render_shared_tool_view, tool_call_id_from_raw,
    tool_command_summary_view, tool_explore_summary_view, tool_identity_from_raw,
    tool_name_from_raw, tool_text_editor,
};
use super::super::utils::{chat_secondary_text_color, is_dark_theme};
use super::assistant_error::assistant_api_error_view;
use super::styles::{ASSISTANT_BLOCK_SPACING, MESSAGE_TEXT_SIZE};
use super::text::{message_editor_body, message_text_body, should_segment_text_block};
use super::think_block::{should_render_think_block, think_block_is_running, think_block_view};
use super::tool_summaries::{
    normalized_visible_text, should_hide_explore_link_box, should_hide_post_explore_tool_block,
    trailing_tool_tail_text_source_block_idx,
};

pub(super) fn count_visible_non_explore_tool_cards(
    blocks: &[ParsedChatBlock],
    tool_block_metadata: &[Option<models::ToolBlockRenderMetadata>],
    tool_last: &HashMap<String, usize>,
) -> usize {
    let mut count = 0usize;
    let mut has_pending_explore_group = false;

    for (block_idx, block) in blocks.iter().enumerate() {
        match block {
            ParsedChatBlock::Think { .. } => {
                has_pending_explore_group = false;
            }
            ParsedChatBlock::Tool { raw } => {
                let metadata = tool_block_metadata.get(block_idx).and_then(Option::as_ref);
                let tool_name = metadata
                    .and_then(|metadata| metadata.name.clone())
                    .or_else(|| tool_name_from_raw(raw));
                let Some(tool_name) = tool_name else {
                    continue;
                };

                if is_explore_tool(&tool_name) {
                    has_pending_explore_group = true;
                    continue;
                }

                let identity = metadata
                    .and_then(|metadata| metadata.identity.clone())
                    .or_else(|| tool_identity_from_raw(raw));
                if let Some(identity) = identity
                    && tool_last.get(&identity).copied() != Some(block_idx)
                {
                    continue;
                }

                if has_pending_explore_group && should_hide_post_explore_tool_block(raw) {
                    has_pending_explore_group = false;
                    continue;
                }

                has_pending_explore_group = false;
                count = count.saturating_add(1);
            }
            ParsedChatBlock::Text { content } => {
                if normalized_visible_text(content).is_some() {
                    has_pending_explore_group = false;
                }
            }
        }
    }

    count
}

fn pending_permission_badge_style(theme: &Theme) -> iced::widget::container::Style {
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
        text_color: Some(chat_secondary_text_color(theme)),
        ..Default::default()
    }
}

fn pending_permission_tool_badge<'a>(
    label: String,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    let label = text(label).size(11).style(|theme: &Theme| iced::widget::text::Style {
        color: Some(if is_dark_theme(theme) {
            Color::from_rgba8(255, 235, 177, 0.96)
        } else {
            Color::from_rgba8(142, 103, 16, 1.0)
        }),
    });

    let content = container(label)
        .padding([2, 8])
        .style(|theme: &Theme| pending_permission_badge_style(theme));

    if let Some(on_press) = on_press {
        button(content)
            .padding(0)
            .style(|theme: &Theme, _status| iced::widget::button::Style {
                background: None,
                border: Border::default(),
                text_color: pending_permission_badge_style(theme)
                    .text_color
                    .unwrap_or_else(|| chat_secondary_text_color(theme)),
                ..Default::default()
            })
            .on_press(on_press)
            .into()
    } else {
        content.into()
    }
}

fn wrap_pending_permission_tool_card<'a>(
    content: Element<'a, Message>,
    label: String,
    request_id: Option<String>,
    is_active: bool,
) -> Element<'a, Message> {
    let on_press = request_id.map(|request_id| {
        Message::Chat(crate::app::message::chat::ChatMessage::PermissionSelectRequest(request_id))
    });

    let card = column![
        row![
            container(Space::new()).width(Length::Fill),
            pending_permission_tool_badge(label, on_press)
        ]
        .align_y(Alignment::Center),
        content
    ]
    .spacing(6);

    if is_active {
        container(card)
            .padding([6, 8])
            .width(Length::Fill)
            .style(|theme: &Theme| {
                let is_dark = is_dark_theme(theme);
                iced::widget::container::Style {
                    background: Some(Background::Color(if is_dark {
                        Color::from_rgba8(61, 49, 19, 0.24)
                    } else {
                        Color::from_rgba8(255, 248, 229, 0.92)
                    })),
                    border: Border {
                        width: 1.0,
                        color: if is_dark {
                            Color::from_rgba8(168, 132, 45, 0.86)
                        } else {
                            Color::from_rgba8(214, 169, 68, 0.96)
                        },
                        radius: 14.0.into(),
                    },
                    ..Default::default()
                }
            })
            .into()
    } else {
        card.into()
    }
}

pub(crate) fn should_highlight_pending_permission_tool(
    matched_request_id: Option<&str>,
    current_request_id: Option<&str>,
    matches_permission_tool: bool,
) -> bool {
    matches_permission_tool
        || matched_request_id.zip(current_request_id).is_some_and(
            |(matched_request_id, current_request_id)| matched_request_id == current_request_id,
        )
}

pub(crate) fn deduped_tool_last_indices(blocks: &[ParsedChatBlock]) -> HashMap<String, usize> {
    deduped_tool_last_indices_with_metadata(blocks, &[])
}

pub(super) fn deduped_tool_last_indices_with_metadata(
    blocks: &[ParsedChatBlock],
    tool_block_metadata: &[Option<models::ToolBlockRenderMetadata>],
) -> HashMap<String, usize> {
    let mut tool_last: HashMap<String, usize> = HashMap::new();

    for (block_idx, block) in blocks.iter().enumerate() {
        let ParsedChatBlock::Tool { raw } = block else {
            continue;
        };

        let metadata = tool_block_metadata.get(block_idx).and_then(Option::as_ref);
        let name =
            metadata.and_then(|metadata| metadata.name.clone()).or_else(|| tool_name_from_raw(raw));
        if let Some(name) = name
            && is_explore_tool(&name)
        {
            continue;
        }

        let identity = metadata
            .and_then(|metadata| metadata.identity.clone())
            .or_else(|| tool_identity_from_raw(raw));
        if let Some(identity) = identity {
            tool_last.insert(identity, block_idx);
        }
    }

    tool_last
}

fn push_explore_summary<'a>(
    mut body: Column<'a, Message>,
    app: &'a App,
    idx: usize,
    explore_group_idx: usize,
    explore_items: &[ExploreItem],
    explore_group_force_running: bool,
    closed_by_following_block: bool,
) -> Column<'a, Message> {
    if let Some(view) = tool_explore_summary_view(
        app,
        idx,
        explore_group_idx,
        explore_items,
        explore_group_force_running,
        closed_by_following_block,
    ) {
        body = body.push(view);
    }
    body
}

fn flush_command_summary<'a>(
    mut body: Column<'a, Message>,
    app: &'a App,
    idx: usize,
    command_items: &mut Vec<CommandItem>,
) -> Column<'a, Message> {
    if let Some(view) = tool_command_summary_view(app, idx, command_items) {
        body = body.push(view);
    }
    command_items.clear();
    body
}

fn push_special_text_block<'a>(
    body: Column<'a, Message>,
    app: &'a App,
    idx: usize,
    text_idx: usize,
    text: String,
    prefer_markdown: bool,
) -> Column<'a, Message> {
    if let Some(view) = assistant_api_error_view(app, idx, Some(text_idx), &text) {
        return body.push(view);
    }

    let special_text_key = ((idx as u64) << 32) | (text_idx as u64);

    // 单一编辑器允许选区跨越换行、段落与列表，避免 Markdown 分块打断复制。
    if let Some(view) = tool_text_editor(
        app,
        ToolTextTarget::SpecialMessageText { msg_idx: idx, text_idx },
        chat_text_font_name(),
        MESSAGE_TEXT_SIZE,
        false,
        false,
    ) {
        return body.push(view);
    }

    if prefer_markdown
        && let Some(blocks) = app.chat_markdown_blocks.get(&special_text_key)
        && !blocks.is_empty()
    {
        return body.push(super::markdown::markdown_body_view(app, idx, text_idx, blocks));
    }

    body.push(message_text_body(text, false))
}

pub(super) fn render_assistant_body<'a>(
    app: &'a App,
    idx: usize,
    think_timing: &'a [models::ThinkTiming],
    special_text_blocks: Vec<String>,
    display_text: String,
    blocks: Vec<ParsedChatBlock>,
    tool_block_metadata: Vec<Option<models::ToolBlockRenderMetadata>>,
    has_special: bool,
    is_streaming_assistant: bool,
    visible_for_copy: &str,
) -> Column<'a, Message> {
    let mut body = column![].spacing(ASSISTANT_BLOCK_SPACING);
    let use_editor = !is_streaming_assistant
        && selectable_text_policy(visible_for_copy, should_segment_text_block(visible_for_copy))
            != SelectableTextPolicy::PreviewWithCopyFallback
        && app.chat_message_editors.get(idx).is_some();

    if has_special {
        let tool_last = deduped_tool_last_indices_with_metadata(&blocks, &tool_block_metadata);
        let message_id = app.chat_message_ids.get(idx).and_then(|value| value.as_deref());
        let is_permission_target_message =
            pending_permission_targets_message(app.permission_modal_request.as_ref(), message_id);
        let permission_badge_text = pending_permission_badge_label(
            &app.permission_modal_requests,
            app.permission_modal_request_id.as_deref(),
        );
        let visible_non_explore_tool_count = if is_permission_target_message {
            count_visible_non_explore_tool_cards(&blocks, &tool_block_metadata, &tool_last)
        } else {
            0
        };
        let total_think_blocks =
            blocks.iter().filter(|block| matches!(block, ParsedChatBlock::Think { .. })).count();

        let trailing_tool_tail_text_block_idx = trailing_tool_tail_text_source_block_idx(&blocks);

        let mut think_idx = 0usize;
        let mut tool_idx = 0usize;
        let mut text_block_idx = 0usize;
        let mut explore_items: Vec<ExploreItem> = Vec::new();
        let mut command_items: Vec<CommandItem> = Vec::new();
        let mut explore_group_idx = 0usize;
        let mut explore_group_force_running = false;
        let mut deferred_text: Option<(usize, String)> = None;

        for (block_i, block) in blocks.iter().enumerate() {
            match block {
                ParsedChatBlock::Think { content, open } => {
                    let timing = think_timing.get(think_idx);
                    let think_running = think_block_is_running(*open, timing);
                    let should_render = should_render_think_block(
                        app.dialogue_flow_show_reasoning_summary,
                        total_think_blocks,
                        *open,
                        timing,
                    );

                    if should_render {
                        body = flush_command_summary(body, app, idx, &mut command_items);
                        body = push_explore_summary(
                            body,
                            app,
                            idx,
                            explore_group_idx,
                            &explore_items,
                            explore_group_force_running,
                            true,
                        );
                        explore_items.clear();
                        explore_group_force_running = false;
                        explore_group_idx = explore_group_idx.saturating_add(1);
                        body = body.push(think_block_view(
                            app,
                            idx,
                            think_idx,
                            content.clone(),
                            *open,
                            timing,
                        ));
                    } else if think_running {
                        // 思考块隐藏时，让前一个探索组继续保持运行态，直到下一个可见的非探索块收口。
                        explore_group_force_running = true;
                    }
                    think_idx += 1;
                }
                ParsedChatBlock::Tool { raw } => {
                    let raw_text = raw.clone();
                    let metadata = tool_block_metadata.get(block_i).and_then(Option::as_ref);
                    let tool_name = metadata
                        .and_then(|metadata| metadata.name.clone())
                        .or_else(|| tool_name_from_raw(&raw_text));

                    if !tool_name.as_deref().is_some_and(is_explore_tool)
                        && let Some(identity) = metadata
                            .and_then(|metadata| metadata.identity.clone())
                            .or_else(|| tool_identity_from_raw(&raw_text))
                        && tool_last.get(&identity).copied() != Some(block_i)
                    {
                        tool_idx += 1;
                        continue;
                    }

                    if let Some(name) = tool_name
                        && is_explore_tool(&name)
                    {
                        body = flush_command_summary(body, app, idx, &mut command_items);
                        explore_items.push(ExploreItem { tool_idx, raw: raw_text.clone() });
                        tool_idx += 1;
                        continue;
                    }

                    let had_explore_items = !explore_items.is_empty();
                    body = push_explore_summary(
                        body,
                        app,
                        idx,
                        explore_group_idx,
                        &explore_items,
                        explore_group_force_running,
                        true,
                    );
                    explore_items.clear();
                    explore_group_force_running = false;
                    explore_group_idx = explore_group_idx.saturating_add(1);
                    if had_explore_items && should_hide_post_explore_tool_block(&raw_text) {
                        tool_idx += 1;
                        continue;
                    }

                    let matched_permission_request = pending_permission_request_for_tool_call(
                        &app.permission_modal_requests,
                        message_id,
                        &raw_text,
                    );

                    let matches_permission_tool = if is_permission_target_message {
                        let raw_call_id = tool_call_id_from_raw(&raw_text);
                        if pending_permission_targets_tool_call(
                            app.permission_modal_request.as_ref(),
                            message_id,
                            &raw_text,
                        ) {
                            true
                        } else if raw_call_id.is_none() {
                            visible_non_explore_tool_count == 1
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                    if is_command_tool(&raw_text)
                        && matched_permission_request.is_none()
                        && !matches_permission_tool
                    {
                        command_items.push(CommandItem { tool_idx, raw: raw_text });
                        tool_idx += 1;
                        continue;
                    }

                    body = flush_command_summary(body, app, idx, &mut command_items);

                    let rendered = render_shared_tool_view(app, idx, tool_idx, &raw_text);

                    if let Some(view) = rendered {
                        let should_highlight_permission_tool =
                            should_highlight_pending_permission_tool(
                                matched_permission_request.map(|request| request.id.as_str()),
                                app.permission_modal_request_id.as_deref(),
                                matches_permission_tool,
                            );
                        let view = if let Some(request) = matched_permission_request {
                            let label = if app.permission_modal_request_id.as_deref()
                                == Some(request.id.as_str())
                            {
                                permission_badge_text.clone()
                            } else {
                                pending_permission_request_badge_label(
                                    &app.permission_modal_requests,
                                    request.id.as_str(),
                                )
                            };
                            wrap_pending_permission_tool_card(
                                view,
                                label,
                                Some(request.id.clone()),
                                should_highlight_permission_tool,
                            )
                        } else if matches_permission_tool {
                            wrap_pending_permission_tool_card(
                                view,
                                permission_badge_text.clone(),
                                None,
                                should_highlight_permission_tool,
                            )
                        } else {
                            view
                        };
                        body = body.push(view);
                    } else {
                        let text = raw_text.trim().to_string();
                        if !text.is_empty() {
                            body = body.push(message_text_body(text, false));
                        }
                    }
                    tool_idx += 1;
                }
                ParsedChatBlock::Text { content } => {
                    // Bug 2 fix: invisible text blocks (e.g. lone "\n") are skipped in
                    // build_render_cache_entry, so skip them here too to avoid consuming
                    // the wrong special_text_blocks slot.
                    if normalized_visible_text(content).is_none() {
                        continue;
                    }
                    body = flush_command_summary(body, app, idx, &mut command_items);
                    let Some(text) = special_text_blocks.get(text_block_idx).cloned() else {
                        continue;
                    };
                    let current_text_idx = text_block_idx;
                    text_block_idx += 1;
                    let had_explore_items = !explore_items.is_empty();

                    // Bug 1 fix: trailing_tool_tail_text_source_block_idx already determined
                    // this text should be deferred regardless of pending explore items.
                    // Flush any accumulated explore items first, then defer the text so it
                    // appears after all trailing explore summaries.
                    if trailing_tool_tail_text_block_idx == Some(block_i) {
                        if had_explore_items {
                            body = push_explore_summary(
                                body,
                                app,
                                idx,
                                explore_group_idx,
                                &explore_items,
                                explore_group_force_running,
                                true,
                            );
                            explore_items.clear();
                            explore_group_force_running = false;
                            explore_group_idx = explore_group_idx.saturating_add(1);
                        }
                        deferred_text = Some((current_text_idx, text));
                        continue;
                    }

                    body = push_explore_summary(
                        body,
                        app,
                        idx,
                        explore_group_idx,
                        &explore_items,
                        explore_group_force_running,
                        true,
                    );
                    explore_items.clear();
                    explore_group_force_running = false;
                    explore_group_idx = explore_group_idx.saturating_add(1);
                    if had_explore_items && should_hide_explore_link_box(&text) {
                        continue;
                    }

                    body = push_special_text_block(
                        body,
                        app,
                        idx,
                        current_text_idx,
                        text,
                        !is_streaming_assistant,
                    );
                }
            }
        }

        body = flush_command_summary(body, app, idx, &mut command_items);
        body = push_explore_summary(
            body,
            app,
            idx,
            explore_group_idx,
            &explore_items,
            explore_group_force_running,
            false,
        );

        if let Some((deferred_idx, deferred_text)) = deferred_text.take() {
            body = push_special_text_block(
                body,
                app,
                idx,
                deferred_idx,
                deferred_text,
                !is_streaming_assistant,
            );
        }

        body
    } else {
        let text = display_text;
        let markdown_key = (idx as u64) << 32;
        if let Some(view) = assistant_api_error_view(app, idx, None, &text) {
            body = body.push(view);
        } else if use_editor {
            // 单一编辑器让选择范围可以跨越 Markdown 段落、列表等块级边界。
            if let Some(view) = message_editor_body(app, idx, app.theme().palette().text) {
                body = body.push(view);
            }
        } else if !is_streaming_assistant
            && let Some(blocks) = app.chat_markdown_blocks.get(&markdown_key)
            && !blocks.is_empty()
        {
            // 编辑器不可用时保留块级 Markdown 与消息复制按钮作为回退。
            body = body.push(super::markdown::markdown_body_view(app, idx, 0, blocks));
        } else if !text.is_empty() {
            body = body.push(message_text_body(text, false));
        }
        body
    }
}
