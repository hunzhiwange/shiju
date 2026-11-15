//! 聊天 UI 的文本编辑器与动画缓存。
use super::*;

pub(super) fn read_only_chat_editor_content(text: &str) -> Option<text_editor::Content> {
    crate::selectable_text::read_only_editor_content(text)
}
pub(super) fn sync_chat_explore_summary_animations_for_message(
    app: &mut ChatUi<'_>,
    msg_idx: usize,
    render_entry: &ChatRenderCacheEntry,
) {
    let now_ms = runtime_time::now_ms();

    for (group_idx, (_, summary_text)) in
        render_entry.explore_summary_text_blocks.iter().enumerate()
    {
        let summary_text = summary_text.render(desktop_i18n::I18n::new(app.desktop_locale));
        let key = explore_summary_animation_key(msg_idx, group_idx);
        match app.chat_explore_summary_animations.entry(key) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                let state = entry.get_mut();
                if state.current_summary_text != summary_text {
                    state.previous_summary_text = state.current_summary_text.clone();
                    state.current_summary_text = summary_text.clone();
                    state.changed_at_ms = Some(now_ms);
                }
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(desktop_chat_tools::tools::explore_input::ExploreSummaryAnimationState {
                    previous_summary_text: summary_text.clone(),
                    current_summary_text: summary_text.clone(),
                    changed_at_ms: None,
                });
            }
        }
    }

    let group_count = render_entry.explore_summary_text_blocks.len();
    app.chat_explore_summary_animations.retain(|key, _| {
        let message_idx = (*key >> 64) as usize;
        let group_idx = (*key & u64::MAX as u128) as usize;
        message_idx != msg_idx || group_idx < group_count
    });
}
pub(super) fn sync_markdown_blocks_for_text(app: &mut ChatUi<'_>, msg_idx: usize, text_idx: usize, text: &str) {
    use crate::message_view::markdown as message_view;

    let blocks = message_view::parse_markdown_blocks(text);
    let block_key = ((msg_idx as u64) << 32) | (text_idx as u64);
    let editor_key = message_view::markdown_block_key(msg_idx, text_idx, 0);
    app.chat_markdown_block_editors.remove(&editor_key);
    if blocks.iter().all(message_view::MarkdownBlock::joins_prose)
        && let Some(editor) =
            read_only_chat_editor_content(&message_view::markdown_editor_text(&blocks))
    {
        app.chat_markdown_block_editors.insert(editor_key, editor);
    }
    if (*app.chat_markdown_selection)
        .as_ref()
        .is_some_and(|selection| selection.message_idx == msg_idx && selection.text_idx == text_idx)
    {
        (*app.chat_markdown_selection) = None;
    }
    app.chat_markdown_blocks.insert(block_key, blocks);
}
pub(super) fn sync_chat_aux_text_editors_for_message(
    app: &mut ChatUi<'_>,
    msg_idx: usize,
    render_entry: &ChatRenderCacheEntry,
    reuse_markdown_cache: bool,
) {
    sync_chat_explore_summary_animations_for_message(app, msg_idx, render_entry);

    let is_assistant_message =
        matches!(app.chat.get(msg_idx).map(|message| message.role), Some(ChatRole::Assistant));

    if !reuse_markdown_cache {
        app.chat_markdown_blocks.retain(|key, _| (*key >> 32) as usize != msg_idx);
        app.chat_markdown_block_editors.retain(|key, _| (*key >> 64) as usize != msg_idx);
    }

    for (text_idx, text) in render_entry.special_text_blocks.iter().enumerate() {
        let key = ((msg_idx as u64) << 32) | (text_idx as u64);
        if let Some(editor) = read_only_chat_editor_content(text) {
            app.chat_special_text_editors.insert(key, editor);
        }
        if is_assistant_message && !reuse_markdown_cache {
            sync_markdown_blocks_for_text(app, msg_idx, text_idx, text);
        }
    }

    // 非特殊助手消息整体走 display_text，按文本块索引 0 缓存 Markdown 块。
    if is_assistant_message
        && !render_entry.has_special_blocks
        && !render_entry.display_text.trim().is_empty()
        && !reuse_markdown_cache
    {
        sync_markdown_blocks_for_text(app, msg_idx, 0, &render_entry.display_text);
    }

    for (tool_idx, source) in render_entry.tool_card_text_blocks.iter().enumerate() {
        let texts = source.render(desktop_i18n::I18n::new(app.desktop_locale));
        for (text_idx, text) in texts.iter().enumerate() {
            if let Some(editor) = read_only_chat_editor_content(text) {
                let key = desktop_chat_tools::tool_text_support::tool_text_key(
                    msg_idx, tool_idx, text_idx,
                );
                app.chat_tool_text_editors.insert(key, editor);
            }
        }
    }

    for (group_tool_idx, summary_text) in &render_entry.explore_summary_text_blocks {
        let summary_text = summary_text.render(desktop_i18n::I18n::new(app.desktop_locale));
        if let Some(editor) = read_only_chat_editor_content(&summary_text) {
            let key = desktop_chat_tools::tool_text_support::tool_text_key(
                msg_idx,
                *group_tool_idx,
                0,
            );
            app.chat_tool_text_editors.insert(key, editor);
        }
    }

    let mut think_idx = 0usize;
    for block in &render_entry.blocks {
        if let ParsedChatBlock::Think { content, .. } = block {
            let normalized =
                crate::message_view::cache_text_support::normalize_display_text(content.trim())
                    .into_owned();
            if let Some(editor) = read_only_chat_editor_content(&normalized) {
                let key = ((msg_idx as u64) << 32) | (think_idx as u64);
                app.chat_think_editors.insert(key, editor);
            }
            think_idx += 1;
        }
    }
}
impl ChatUi<'_> {
    pub fn sync_completed_stream_text(&mut self, msg_idx: usize) {
        let Some(message) = self.chat.get(msg_idx) else { return };
        let entry = crate::message_view::render_cache::build_render_cache_entry(
            &message.content,
            "",
            0,
            self.dialogue_flow_show_reasoning_summary,
        );
        let mut text_idx = 0;
        let mut think_idx = 0;
        for (block_idx, block) in entry.blocks.iter().enumerate() {
            match block {
                ParsedChatBlock::Text { content } => {
                    let cleaned =
                        crate::message_view::cache_text_support::strip_internal_tool_trace(
                            content,
                        );
                    if crate::message_view::cache_text_support::normalize_display_text(
                        cleaned.trim(),
                    )
                    .is_empty()
                    {
                        continue;
                    }
                    let current_idx = text_idx;
                    text_idx += 1;
                    if block_idx + 1 == entry.blocks.len() {
                        continue;
                    }
                    let Some(text) = entry.special_text_blocks.get(current_idx) else { continue };
                    let key = ((msg_idx as u64) << 32) | current_idx as u64;
                    if (*self.chat_special_text_editors)
                        .get(&key)
                        .is_some_and(|editor| editor.text() == *text)
                    {
                        continue;
                    }
                    if let Some(editor) = read_only_chat_editor_content(text) {
                        self.chat_special_text_editors.insert(key, editor);
                    }
                    sync_markdown_blocks_for_text(self, msg_idx, current_idx, text);
                }
                ParsedChatBlock::Think { content, open } => {
                    let key = ((msg_idx as u64) << 32) | think_idx as u64;
                    think_idx += 1;
                    if *open {
                        continue;
                    }
                    let text = crate::message_view::cache_text_support::normalize_display_text(
                        content.trim(),
                    );
                    if self.chat_think_editors.get(&key).is_some_and(|editor| editor.text() == text)
                    {
                        continue;
                    }
                    if let Some(editor) = read_only_chat_editor_content(&text) {
                        self.chat_think_editors.insert(key, editor);
                    }
                }
                _ => {}
            }
        }
    }
}

impl ChatUi<'_> {
    pub fn finalize_stream_text(&mut self) {
        let Some(idx) = self.chat.iter().rposition(|message| message.role == ChatRole::Assistant)
        else {
            return;
        };
        let phase = prepare_chat_ui_message_phase(
            &self.chat[idx],
            idx,
            self.dialogue_flow_show_reasoning_summary,
        );
        self.apply_prepared_chat_ui_phase(phase);
    }
}
