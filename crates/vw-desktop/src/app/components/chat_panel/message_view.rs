//! Host snapshot assembly and cross-domain message routing.
mod markdown_links;
mod markdown_selection;
pub(in crate::app::components) use markdown_selection::selectable_titled_text;
mod parse;
pub(crate) use parse::assistant_speech_text;
mod render_cache;
mod styles;

use crate::app::models::{self, ChatRole};
use crate::app::{App, Message};
use iced::Element;
use session_ui_types::ProcessingTiming;
use std::borrow::Cow;
mod snapshot;
pub(crate) use self::parse::hash_chat_content;
pub(crate) use self::render_cache::{
    RevisionRenderKey, assistant_render_blocks, build_render_cache_entry,
    effective_assistant_render_cache,
};
pub(crate) use self::styles::{
    compaction_status_card_style, compaction_status_text_color, message_surface_contract,
    message_surface_style,
};
pub(crate) use desktop_chat_view::message_view::assistant_body::deduped_tool_last_indices;
pub(crate) use desktop_chat_view::message_view::contracts::*;
pub(crate) use desktop_chat_view::message_view::markdown::{
    MarkdownBlock, markdown_block_key, markdown_editor_text, parse_markdown_blocks,
};
#[cfg(test)]
pub(crate) use desktop_chat_view::message_view::permission_card::should_highlight_pending_permission_tool;
pub(crate) use desktop_chat_view::message_view::text::estimate_message_height_rough;
#[cfg(test)]
pub(crate) use desktop_chat_view::message_view::text::should_prefer_plain_think_body;
#[cfg(test)]
pub(crate) use desktop_chat_view::message_view::think_block::{
    should_render_think_block, think_block_default_expanded, think_block_resolved_expanded,
};
#[cfg(test)]
pub(crate) use desktop_chat_view::message_view::tool_summaries::explore_summary_text_blocks;
#[cfg(test)]
pub(crate) use desktop_chat_view::message_view::tool_summaries::summarize_explore_items;
#[cfg(test)]
pub(crate) use desktop_chat_view::message_view::tool_summaries::tool_card_text_blocks;

pub(crate) fn chat_message_action_text(
    app: &App,
    idx: usize,
    prefer_selection: bool,
) -> Option<String> {
    desktop_chat_view::message_view::bubble::chat_message_action_text(
        &snapshot::snapshot(app),
        idx,
        prefer_selection,
    )
}
pub fn message_view<'a>(
    app: &'a App,
    idx: usize,
    role: ChatRole,
    content: &'a str,
    think_timing: &'a [models::ThinkTiming],
    processing_timing: Option<&ProcessingTiming>,
    message_meta: Option<Cow<'a, str>>,
    render_cache: &'a models::ChatRenderCacheEntry,
    heavy: bool,
) -> Element<'a, Message> {
    let requesting = app
        .current_session_runtime_ref()
        .is_some_and(|runtime| runtime.request_lifecycle.is_requesting);
    message_view_with_runtime(
        app,
        idx,
        role,
        content,
        think_timing,
        processing_timing,
        message_meta,
        render_cache,
        heavy,
        requesting,
        None,
    )
}
pub(crate) fn message_view_with_runtime<'a>(
    app: &'a App,
    idx: usize,
    role: ChatRole,
    content: &'a str,
    think_timing: &'a [models::ThinkTiming],
    processing_timing: Option<&ProcessingTiming>,
    message_meta: Option<Cow<'a, str>>,
    render_cache: &'a models::ChatRenderCacheEntry,
    heavy: bool,
    requesting: bool,
    revision: Option<&RevisionRenderKey>,
) -> Element<'a, Message> {
    desktop_chat_view::message_view::bubble::message_view_with_runtime(
        &snapshot::snapshot(app),
        idx,
        role,
        content,
        think_timing,
        processing_timing,
        message_meta,
        render_cache,
        heavy,
        requesting,
        revision,
    )
}
pub(crate) use desktop_chat_view::message_view::recovery::{
    localized_model_recovery_text, model_recovery_text,
};

pub(crate) fn recovery_notice_text(app: &App) -> Option<String> {
    let runtime = app.current_session_runtime_ref()?;
    desktop_chat_view::message_view::recovery::recovery_notice_text(
        crate::app::i18n::I18n::new(app.general_settings.desktop_locale),
        runtime.request_lifecycle.active_agent_request.as_ref().map(|request| request.id),
        runtime.model_recovery.as_ref().map(|(id, event)| (*id, event)),
    )
}

pub(crate) fn recovery_notice_view(app: &App) -> Element<'_, Message> {
    desktop_chat_view::message_view::recovery::recovery_notice_view(recovery_notice_text(app))
}

#[cfg(test)]
mod host_contract_tests;

#[cfg(test)]
pub(super) fn should_use_user_message_editor(
    app: &App,
    idx: usize,
    content: &str,
    no_attachments: bool,
) -> bool {
    desktop_chat_view::message_view::bubble::should_use_user_message_editor(
        &snapshot::snapshot(app),
        idx,
        content,
        no_attachments,
    )
}
