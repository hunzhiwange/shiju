//! 助手消息主体渲染。
//!
//! 该模块负责把已经解析好的助手消息块拼装为最终 UI，包含思考块、工具卡、
//! 特殊文本与探索摘要的组织顺序。

use std::collections::HashMap;

use iced::Element;
use iced::widget::{Column, column};

use super::ParsedChatBlock;
use super::markdown_selection::SelectionMessage;
use super::render_snapshot::{Action, MessageViewSnapshot};
use session_ui_types as models;

use super::assistant_error_render::assistant_api_error_view;
use super::render_support::{
    message_editor_body, message_text_body, render_shared_tool_view,
    think_block_view_with_streaming, tool_command_summary_view, tool_explore_summary_view,
    tool_text_editor,
};
use super::styles::{ASSISTANT_BLOCK_SPACING, MESSAGE_TEXT_SIZE};
use super::text::should_segment_text_block;
use super::think_block::{should_render_think_block, think_block_is_running};
use super::tool_summaries::{
    normalized_visible_text, should_hide_explore_link_box, should_hide_post_explore_tool_block,
};
use crate::selectable_text::{SelectableTextPolicy, selectable_text_policy};
use desktop_chat_tools::tool_text_support::ToolTextTarget;
use desktop_chat_tools::tool_text_support::chat_text_font_name;
use desktop_chat_tools::tools::{
    command_summary_view::is_command_tool,
    tool_parse::tool_call_id_from_raw,
    tool_permission::{
        pending_permission_badge_label, pending_permission_request_badge_label,
        pending_permission_request_for_tool_call, pending_permission_targets_message,
        pending_permission_targets_tool_call,
    },
    types::{CommandItem, ExploreItem},
};

pub fn count_visible_non_explore_tool_cards(
    blocks: &[ParsedChatBlock],
    metadata: &[Option<crate::render_cache::ToolBlockRenderMetadata>],
    last: &HashMap<String, usize>,
) -> usize {
    use super::assistant_body::{BlockPresentation, count_visible_non_explore_tool_cards};
    let presentation: Vec<_> = blocks
        .iter()
        .map(|block| {
            let mut facts = BlockPresentation::default();
            match block {
                ParsedChatBlock::Text { content } => {
                    facts.visible_text = normalized_visible_text(content).is_some()
                }
                ParsedChatBlock::Tool { raw } => {
                    facts.hide_after_explore = should_hide_post_explore_tool_block(raw)
                }
                ParsedChatBlock::Think { .. } => {}
            }
            facts
        })
        .collect();
    count_visible_non_explore_tool_cards(blocks, metadata, last, &presentation)
}

pub(crate) use super::permission_card::should_highlight_pending_permission_tool;
fn wrap_pending_permission_tool_card<'a, M: Clone + 'static>(
    app: &MessageViewSnapshot<'a, M>,
    content: Element<'a, M>,
    label: String,
    request_id: Option<String>,
    is_active: bool,
) -> Element<'a, M> {
    let on_press = request_id.map(|id| (app.emit)(Action::PermissionSelectRequest(id)));
    super::permission_card::wrap_pending_permission_tool_card(content, label, on_press, is_active)
}

pub(crate) use super::assistant_body::deduped_tool_last_indices_with_metadata;

fn push_explore_summary<'a, M: SelectionMessage + 'static>(
    mut body: Column<'a, M>,
    app: &MessageViewSnapshot<'a, M>,
    idx: usize,
    explore_group_idx: usize,
    explore_items: &[ExploreItem],
    explore_group_force_running: bool,
    closed_by_following_block: bool,
) -> Column<'a, M> {
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

fn flush_command_summary<'a, M: SelectionMessage + 'static>(
    mut body: Column<'a, M>,
    app: &MessageViewSnapshot<'a, M>,
    idx: usize,
    command_items: &mut Vec<CommandItem>,
) -> Column<'a, M> {
    if let Some(view) = tool_command_summary_view(app, idx, command_items) {
        body = body.push(view);
    }
    command_items.clear();
    body
}

fn push_special_text_block<'a, M: SelectionMessage + 'static>(
    body: Column<'a, M>,
    app: &MessageViewSnapshot<'a, M>,
    idx: usize,
    text_idx: usize,
    text: String,
    prefer_markdown: bool,
) -> Column<'a, M> {
    if let Some(view) = assistant_api_error_view(app, idx, Some(text_idx), &text) {
        return body.push(view);
    }

    let special_text_key = ((idx as u64) << 32) | (text_idx as u64);

    if prefer_markdown
        && let Some(blocks) = app.chat_markdown_blocks.get(&special_text_key)
        && !blocks.is_empty()
    {
        return body.push(super::render_support::markdown_body_view(app, idx, text_idx, blocks));
    }

    if prefer_markdown
        && let Some(view) = tool_text_editor(
            app,
            ToolTextTarget::SpecialMessageText { msg_idx: idx, text_idx },
            chat_text_font_name(),
            MESSAGE_TEXT_SIZE,
            false,
            false,
        )
    {
        return body.push(view);
    }

    body.push(message_text_body(app, text, false))
}

pub fn render_assistant_body<'a, M: SelectionMessage + 'static, T, U, V, W>(
    app: &MessageViewSnapshot<'a, M>,
    idx: usize,
    think_timing: &'a [models::ThinkTiming],
    special_text_blocks: T,
    display_text: U,
    blocks: V,
    tool_block_metadata: W,
    has_special: bool,
    is_streaming_assistant: bool,
    visible_for_copy: &str,
) -> Column<'a, M>
where
    T: AsRef<[String]>,
    U: AsRef<str>,
    V: AsRef<[ParsedChatBlock]>,
    W: AsRef<[Option<crate::render_cache::ToolBlockRenderMetadata>]>,
{
    let special_text_blocks = special_text_blocks.as_ref();
    let display_text = display_text.as_ref();
    let blocks = blocks.as_ref();
    let tool_block_metadata = tool_block_metadata.as_ref();
    let mut body = column![].spacing(ASSISTANT_BLOCK_SPACING);
    let use_editor = !is_streaming_assistant
        && selectable_text_policy(visible_for_copy, should_segment_text_block(visible_for_copy))
            != SelectableTextPolicy::PreviewWithCopyFallback
        && app.text.chat_message_editors.get(idx).is_some();

    if has_special {
        let tool_last = deduped_tool_last_indices_with_metadata(blocks, tool_block_metadata);
        let message_id = app.chat_message_ids.get(idx).and_then(|value| value.as_deref());
        let is_permission_target_message =
            pending_permission_targets_message(app.permission_modal_request, message_id);
        let permission_badge_text = pending_permission_badge_label(
            desktop_i18n::I18n::new(app.desktop_locale),
            app.permission_modal_requests,
            app.permission_modal_request_id.as_deref(),
        );
        let visible_non_explore_tool_count = if is_permission_target_message {
            count_visible_non_explore_tool_cards(blocks, tool_block_metadata, &tool_last)
        } else {
            0
        };
        let total_think_blocks =
            blocks.iter().filter(|block| matches!(block, ParsedChatBlock::Think { .. })).count();

        use super::assistant_body::{BlockPresentation, BodyPart, assistant_body_sequence};
        let mut think_idx = 0;
        let mut text_idx = 0;
        let presentation: Vec<_> = blocks
            .iter()
            .map(|block| {
                let mut facts = BlockPresentation::default();
                match block {
                    ParsedChatBlock::Think { open, .. } => {
                        let timing = think_timing.get(think_idx);
                        facts.think_running = think_block_is_running(*open, timing);
                        facts.show_think = should_render_think_block(
                            app.dialogue_flow_show_reasoning_summary,
                            total_think_blocks,
                            *open,
                            timing,
                        );
                        think_idx += 1;
                    }
                    ParsedChatBlock::Tool { raw } => {
                        let raw_text = raw;
                        let matched_permission_request = pending_permission_request_for_tool_call(
                            app.permission_modal_requests,
                            message_id,
                            &raw_text,
                        );

                        let matches_permission_tool = if is_permission_target_message {
                            let raw_call_id = tool_call_id_from_raw(&raw_text);
                            if pending_permission_targets_tool_call(
                                app.permission_modal_request,
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

                        facts.hide_after_explore = should_hide_post_explore_tool_block(raw);
                        facts.group_command = is_command_tool(raw)
                            && matched_permission_request.is_none()
                            && !matches_permission_tool;
                    }
                    ParsedChatBlock::Text { content } => {
                        facts.visible_text = normalized_visible_text(content).is_some();
                        if facts.visible_text {
                            if let Some(text) = special_text_blocks.get(text_idx) {
                                facts.hide_after_explore = should_hide_explore_link_box(text);
                                text_idx += 1;
                            }
                        }
                    }
                }
                facts
            })
            .collect();
        for part in assistant_body_sequence(
            blocks,
            tool_block_metadata,
            &presentation,
            special_text_blocks.len(),
        ) {
            match part {
                BodyPart::Think { block_idx, think_idx } => {
                    let ParsedChatBlock::Think { content, open } = &blocks[block_idx] else {
                        unreachable!()
                    };
                    body = body.push(think_block_view_with_streaming(
                        app,
                        idx,
                        think_idx,
                        content.clone(),
                        *open,
                        think_timing.get(think_idx),
                        is_streaming_assistant,
                    ));
                }
                BodyPart::Text { block_idx, text_idx } => {
                    body = push_special_text_block(
                        body,
                        app,
                        idx,
                        text_idx,
                        special_text_blocks[text_idx].clone(),
                        !is_streaming_assistant || block_idx + 1 < blocks.len(),
                    );
                }
                BodyPart::Commands(mut items) => {
                    body = flush_command_summary(body, app, idx, &mut items);
                }
                BodyPart::Explore { group_idx, items, force_running, closed } => {
                    body = push_explore_summary(
                        body,
                        app,
                        idx,
                        group_idx,
                        &items,
                        force_running,
                        closed,
                    );
                }
                BodyPart::Tool { block_idx, tool_idx } => {
                    let ParsedChatBlock::Tool { raw: raw_text } = &blocks[block_idx] else {
                        unreachable!()
                    };
                    let matched_permission_request = pending_permission_request_for_tool_call(
                        app.permission_modal_requests,
                        message_id,
                        &raw_text,
                    );

                    let matches_permission_tool = if is_permission_target_message {
                        let raw_call_id = tool_call_id_from_raw(&raw_text);
                        if pending_permission_targets_tool_call(
                            app.permission_modal_request,
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
                                    desktop_i18n::I18n::new(app.desktop_locale),
                                    app.permission_modal_requests,
                                    request.id.as_str(),
                                )
                            };
                            wrap_pending_permission_tool_card(
                                app,
                                view,
                                label,
                                Some(request.id.clone()),
                                should_highlight_permission_tool,
                            )
                        } else if matches_permission_tool {
                            wrap_pending_permission_tool_card(
                                app,
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
                            body = body.push(message_text_body(app, text, false));
                        }
                    }
                }
            }
        }
        body
    } else {
        let text = display_text;
        let markdown_key = (idx as u64) << 32;
        if let Some(view) = assistant_api_error_view(app, idx, None, &text) {
            body = body.push(view);
        } else if !is_streaming_assistant
            && let Some(blocks) = app.chat_markdown_blocks.get(&markdown_key)
            && !blocks.is_empty()
        {
            body = body.push(super::render_support::markdown_body_view(app, idx, 0, blocks));
        } else if use_editor {
            if let Some(view) = message_editor_body(app, idx, app.theme().palette().text) {
                body = body.push(view);
            }
        } else if !text.is_empty() {
            body = body.push(message_text_body(app, text.to_owned(), false));
        }
        body
    }
}
