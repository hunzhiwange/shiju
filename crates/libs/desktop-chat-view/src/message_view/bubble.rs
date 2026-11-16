//! Message bubble layout over borrowed session view data.
use super::assistant_render::render_assistant_body;
use super::contracts::*;
use super::markdown_selection::SelectionMessage;
use super::parse::hash_chat_content;
use super::render_cache::{assistant_render_blocks, effective_assistant_render_cache};
use super::render_snapshot::{Action, MessageViewSnapshot};
use super::render_support::{
    message_text_body, session_control_selection_card, user_message_editor_body,
};
use super::styles::*;
use super::text::{
    split_session_control_selection, task_session_visible_text, user_message_preview,
};
use super::text_normalize::normalize_display_text;
use crate::render_cache::RevisionRenderKey;
use crate::selectable_text::{SelectableTextPolicy, selectable_text_policy};
use crate::utils::ui::copy_tooltip_content;
use desktop_assets::assets::Icon;
use desktop_chat_tools::card_style::{chat_context_target_key, icon_svg};
use desktop_chat_tools::tool_text_support::{chat_text_font, selected_chat_text_for_message};
use desktop_chat_tools::tools::tool_permission::{
    pending_permission_badge_label, pending_permission_request_badge_label,
    pending_permission_request_for_message, pending_permission_targets_message,
};
use desktop_ui_controls::right_click_area::RightClickArea;
use iced::widget::{button, column, container, row, text as iced_text};
use iced::{Alignment, Element, Length, Theme};
use session_ui_types::{ChatRole, ProcessingTiming};
use std::borrow::Cow;
fn is_recent_copy<M>(app: &MessageViewSnapshot<'_, M>, hash: u64) -> bool {
    app.copied_recently && app.last_copied_code_hash == Some(hash)
}
fn resolve_visible_text_and_copy_hash<'a, M>(
    app: &MessageViewSnapshot<'a, M>,
    idx: usize,
    content: &str,
    hash: u64,
    cache: &crate::render_cache::ChatRenderCacheEntry,
) -> (Cow<'a, str>, u64) {
    super::render_cache::resolve_visible_text_and_copy_hash(
        app.chat_visible_text_cache.get(idx).and_then(Option::as_deref),
        app.chat_copy_hash_cache.get(idx).and_then(|h| *h),
        content,
        hash,
        cache,
    )
}
pub fn chat_message_action_text<M: SelectionMessage>(
    app: &MessageViewSnapshot<'_, M>,
    idx: usize,
    prefer_selection: bool,
) -> Option<String> {
    if prefer_selection && let Some(selected) = selected_chat_text_for_message(&app.text, idx) {
        return Some(selected);
    }

    let message = app.chat.get(idx)?;
    let task_session_content = if message.role == ChatRole::User {
        task_session_visible_text(&message.content)
    } else {
        Cow::Borrowed(message.content.as_str())
    };
    let content = task_session_content.as_ref();
    let content_hash = hash_chat_content(content);
    let fallback_cache = crate::render_cache::ChatRenderCacheEntry::default();
    let render_cache = app.chat_render_cache.get(&idx).unwrap_or(&fallback_cache);
    let (visible_text, _) =
        resolve_visible_text_and_copy_hash(app, idx, content, content_hash, render_cache);

    if prefer_selection && visible_text.trim().is_empty() {
        Some(content.to_string())
    } else {
        Some(visible_text.into_owned())
    }
}

pub fn copy_chat_message_action(session_id: &str, message_idx: usize) -> Action {
    Action::CopyChatMessage { session_id: session_id.to_string(), message_idx }
}

pub fn open_chat_message_context_menu_action(
    target: u64,
    point: iced::Point,
    session_id: &str,
    message_idx: usize,
) -> Action {
    Action::OpenChatMessageContextMenu {
        target,
        x: point.x,
        y: point.y,
        session_id: session_id.to_string(),
        message_idx,
    }
}

pub fn message_view_with_runtime<'a, M: SelectionMessage + 'static>(
    app: &MessageViewSnapshot<'a, M>,
    idx: usize,
    role: ChatRole,
    content: &str,
    think_timing: &'a [session_ui_types::ThinkTiming],
    processing_timing: Option<&ProcessingTiming>,
    message_meta: Option<Cow<'a, str>>,
    render_cache: &'a crate::render_cache::ChatRenderCacheEntry,
    _enable_heavy_tool_content: bool,
    is_requesting: bool,
    expected_revision_key: Option<&RevisionRenderKey>,
) -> Element<'a, M> {
    if role == ChatRole::System
        && (compaction_status_card_contract(content).is_some() || content == TODO_CONTINUATION_TEXT)
    {
        return container(
            container(
                iced_text(desktop_i18n::I18n::new(app.desktop_locale).__(content).to_string())
                    .size(MESSAGE_META_TEXT_SIZE)
                    .style(|theme: &Theme| iced::widget::text::Style {
                        color: Some(compaction_status_text_color(theme)),
                    }),
            )
            .padding([8, 14])
            .style(compaction_status_card_style),
        )
        .width(Length::Fill)
        .height(Length::Fixed(44.0))
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .into();
    }
    if role == ChatRole::System && app.recovery_text.as_deref() == Some(content) {
        return container(
            iced_text(app.localized_recovery_text.clone().unwrap_or_default())
                .size(MESSAGE_META_TEXT_SIZE),
        )
        .padding([8, 14])
        .width(Length::Fill)
        .into();
    }
    let task_session_content = if role == ChatRole::User {
        task_session_visible_text(content)
    } else {
        Cow::Borrowed(content)
    };
    let content = task_session_content.as_ref();
    let is_user = role == ChatRole::User;
    let assistant_like = matches!(role, ChatRole::Assistant | ChatRole::Tool);
    let is_streaming_assistant =
        role == ChatRole::Assistant && is_requesting && idx + 1 == app.chat.len();
    let live_processing_elapsed_ms =
        is_streaming_assistant.then_some(app.live_processing_elapsed_ms).flatten();
    let cache_matches_revision = expected_revision_key.is_none_or(|expected| {
        render_cache.revision_key.as_ref().is_some_and(|actual| actual.matches_key(expected))
    });
    let content_hash = if render_cache.content_hash == 0 || !cache_matches_revision {
        #[cfg(test)]
        crate::performance::record_full_content_hash(content.len());
        hash_chat_content(content)
    } else {
        render_cache.content_hash
    };
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
        assistant_blocks.iter().any(|block| matches!(block, super::ParsedChatBlock::Tool { .. }));
    let attachments = (app.attachments)(content);
    let cleaned_attachment_text = &attachments.text;
    let prefer_attachment_card_render =
        attachments.has_attachments && !has_special_assistant_blocks;

    let recently_copied = is_recent_copy(app, visible_content_hash);
    let copy_icon = if recently_copied { Icon::Check } else { Icon::Copy };
    let copy_tip = if recently_copied {
        desktop_i18n::I18n::new(app.desktop_locale).__("已复制")
    } else {
        desktop_i18n::I18n::new(app.desktop_locale).__("复制消息")
    };
    let permission_badge_text = pending_permission_badge_label(
        desktop_i18n::I18n::new(app.desktop_locale),
        app.permission_modal_requests,
        app.permission_modal_request_id.as_deref(),
    );
    let message_id = app.chat_message_ids.get(idx).and_then(|message_id| message_id.as_deref());
    let supports_permission_badge =
        role == ChatRole::Tool || (role == ChatRole::Assistant && assistant_has_tool_blocks);
    let is_permission_target =
        pending_permission_targets_message(app.permission_modal_request, message_id)
            && supports_permission_badge;
    let matched_permission_request = supports_permission_badge
        .then(|| pending_permission_request_for_message(app.permission_modal_requests, message_id))
        .flatten();
    let permission_badge: Option<(String, Option<M>)> = if is_permission_target {
        Some((
            permission_badge_text.clone(),
            app.permission_modal_request_id.map(|request_id| {
                (app.emit)(Action::PermissionSelectRequest(request_id.to_owned()))
            }),
        ))
    } else {
        matched_permission_request.map(|request| {
            (
                pending_permission_request_badge_label(
                    desktop_i18n::I18n::new(app.desktop_locale),
                    app.permission_modal_requests,
                    request.id.as_str(),
                ),
                Some((app.emit)(Action::PermissionSelectRequest(request.id.clone()))),
            )
        })
    };
    let can_branch_or_reset = is_user && app.active_session_id.is_some();

    let icon_action_btn = |icon, label: &'static str, on_press, highlighted, compact_square| {
        super::layout::icon_action_button(
            icon,
            copy_tooltip_content(label),
            on_press,
            highlighted,
            compact_square,
            is_user,
        )
    };

    let session_id = app.active_session_id.unwrap_or_default();
    let copy_message = (app.emit)(copy_chat_message_action(session_id, idx));

    let fork_press = (app.emit)(Action::Fork(idx));
    let reset_press = (app.emit)(Action::Reset(idx));

    let user_text = if is_user {
        let (text, _) = split_session_control_selection(cleaned_attachment_text.trim());
        normalize_display_text(text.trim()).into_owned()
    } else {
        String::new()
    };
    let user_preview = user_message_preview(&user_text);
    let user_expanded = app.chat_message_expanded.contains(&idx);
    let user_collapsed = user_preview.is_some() && !user_expanded;
    let plain_text_body = |text: String| {
        if user_collapsed {
            message_text_body(app, format!("{}…", user_preview.unwrap_or_default()), true)
        } else {
            message_text_body(app, text, true)
        }
    };

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
        let use_editor = !user_collapsed
            && should_use_user_message_editor(app, idx, content, !attachments.has_attachments);
        let (text, selection) = split_session_control_selection(cleaned_attachment_text.trim());
        if use_editor {
            if let Some(view) = user_message_editor_body(app, idx, app.theme().palette().text) {
                body = body.push(view);
            }
        } else {
            let text = normalize_display_text(text.trim()).into_owned();
            if !text.is_empty() {
                body = body.push(plain_text_body(text));
            }
        }
        if let Some(selection) = selection {
            body = body.push(session_control_selection_card(
                app,
                desktop_i18n::I18n::new(app.desktop_locale),
                selection,
            ));
        }
        body
    };
    let body = if prefer_attachment_card_render {
        let mut attachment_body = column![].spacing(8);
        if attachments.has_attachments {
            attachment_body = attachment_body.push((attachments.preview)());
        }
        let (text, selection) = split_session_control_selection(cleaned_attachment_text.trim());
        let text = normalize_display_text(text.trim()).into_owned();
        if !text.is_empty() {
            attachment_body = attachment_body.push(plain_text_body(text));
        }
        if let Some(selection) = selection {
            attachment_body = attachment_body.push(session_control_selection_card(
                app,
                desktop_i18n::I18n::new(app.desktop_locale),
                selection,
            ));
        }
        attachment_body
    } else if attachments.has_attachments && !assistant_like {
        let mut attachment_body = column![(attachments.preview)()].spacing(8);
        let (text, selection) = split_session_control_selection(cleaned_attachment_text.trim());
        let text = normalize_display_text(text.trim()).into_owned();
        if !text.is_empty() {
            attachment_body = attachment_body.push(plain_text_body(text));
        }
        if let Some(selection) = selection {
            attachment_body = attachment_body.push(session_control_selection_card(
                app,
                desktop_i18n::I18n::new(app.desktop_locale),
                selection,
            ));
        }
        attachment_body
    } else {
        body
    };
    let body = if user_preview.is_some() {
        let i18n = desktop_i18n::I18n::new(app.desktop_locale);
        let toggle = button(
            row![
                iced_text(i18n.__(if user_expanded { "收起" } else { "查看更多" })).size(13),
                icon_svg(if user_expanded { Icon::ChevronUp } else { Icon::ChevronDown })
                    .width(Length::Fixed(14.0))
                    .height(Length::Fixed(14.0)),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .padding([6, 8])
        .style(desktop_ui_controls::system_settings_common::rounded_action_btn_style)
        .on_press((app.emit)(Action::ToggleUserMessage(idx)));
        body.push(toggle)
    } else {
        body
    };
    let body = if assistant_like {
        let mut assistant_content = column![].spacing(ASSISTANT_HEADER_BODY_SPACING);
        if let Some(label) = processing_duration_label(
            desktop_i18n::I18n::new(app.desktop_locale),
            processing_timing,
            live_processing_elapsed_ms,
        ) {
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

    let emit = app.emit.clone();
    let bubble_inner: Element<'a, M> = RightClickArea::new(
        bubble_inner.into(),
        Box::new(move |point| {
            emit(open_chat_message_context_menu_action(context_key, point, session_id, idx))
        }),
    )
    .use_global_position()
    .preserve_on_right_click()
    .into();

    let meta = message_meta.map(|meta_text| {
        iced_text(meta_text)
            .size(MESSAGE_META_TEXT_SIZE)
            .font(chat_text_font())
            .style(move |theme: &Theme| iced::widget::text::Style {
                color: Some(message_meta_text_color(theme, is_user)),
            })
            .into()
    });
    let badge = permission_badge.map(|(label, on_press)| {
        super::layout::approval_badge_view(label, on_press, chat_text_font())
    });
    let mut actions = Vec::new();
    if can_branch_or_reset {
        actions.push(icon_action_btn(
            Icon::GitBranch,
            desktop_i18n::I18n::new(app.desktop_locale).__("分叉到新会话"),
            fork_press,
            false,
            true,
        ));
        actions.push(icon_action_btn(
            Icon::ArrowCounterClockwise,
            desktop_i18n::I18n::new(app.desktop_locale).__("重置到此点"),
            reset_press,
            false,
            true,
        ));
    }
    actions.push(icon_action_btn(copy_icon, copy_tip, copy_message, recently_copied, true));
    let footer = super::layout::message_footer(meta, badge, actions, is_user);
    super::layout::message_layout(bubble_inner, footer, is_user)
}

pub fn should_use_user_message_editor<M>(
    app: &MessageViewSnapshot<'_, M>,
    idx: usize,
    content: &str,
    has_no_attachments: bool,
) -> bool {
    has_no_attachments
        && selectable_text_policy(content, false) != SelectableTextPolicy::PreviewWithCopyFallback
        && app
            .text
            .chat_message_editors
            .get(idx)
            .is_some_and(|editor| !editor.text().trim().is_empty())
}
