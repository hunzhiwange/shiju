//! 聊天 UI 的预处理结果接纳与失效。
use super::*;

impl ChatUi<'_> {
    pub fn invalidate_chat_ui_state(&mut self) {
        self.chat_render_cache.clear();
        self.chat_visible_text_cache.clear();
        self.chat_copy_hash_cache.clear();
        self.message_meta_texts.clear();
        self.user_question_markers.clear();
        (*self.user_question_indexed_chat_len) = 0;
        self.prepared_chat_ui_chunks.clear();
        self.preparing_chat_ui_chunks.clear();
        self.dirty_preparing_chat_ui_chunks.clear();
        (*self.last_visited_chat_ui_chunk_start) = None;
        (*self.pinned_chat_ui_chunk_start) = None;
        self.chat_message_expanded.clear();
        self.chat_height_index.clear();
        self.chat_virtualizer.set_estimated_heights(&[]);
        self.sync_chat_virtualizer_compat_fields();
        self.chat_message_measured_heights.clear();
        self.chat_message_editors.clear();
        self.chat_special_text_editors.clear();
        self.chat_markdown_blocks.clear();
        (*self.chat_markdown_selection) = None;
        self.chat_markdown_block_editors.clear();
        self.chat_tool_text_editors.clear();
        self.chat_think_editors.clear();
        self.chat_think_expanded.clear();
        self.chat_think_collapsed.clear();
        (*self.chat_think_hovered_idx) = None;
        self.chat_tool_file_expanded.clear();
        (*self.chat_tool_file_hovered) = None;
        self.chat_tool_expanded.clear();
        (*self.chat_tool_hovered_idx) = None;
        self.chat_explore_expanded.clear();
        self.chat_explore_summary_animations.clear();
        (*self.tool_detail_dialog) = None;
        self.chat_think_scroll_ids.clear();
        (*self.chat_reset_menu_idx) = None;
        self.rebuild_chat_message_estimated_heights();
    }
}

impl ChatUi<'_> {
    pub fn apply_prepared_chat_ui_phase(&mut self, phase: PreparedChatUiPhase) {
        let chunk = match phase {
            PreparedChatUiPhase::Base(chunk) | PreparedChatUiPhase::Chunk(chunk) => chunk,
            PreparedChatUiPhase::Message(message) => {
                self.apply_prepared_chat_ui_message(message);
                return;
            }
        };

        let PreparedChatUiChunk {
            chunk_start_idx,
            chunk_end_idx,
            render_cache,
            visible_texts,
            copy_hashes,
            message_editor_texts,
        } = chunk;

        let chunk_end_idx = chunk_end_idx.min(self.chat.len());

        self.preparing_chat_ui_chunks.remove(&chunk_start_idx);
        self.dirty_preparing_chat_ui_chunks.remove(&chunk_start_idx);
        if self.chat_visible_text_cache.len() < self.chat.len() {
            self.chat_visible_text_cache.resize(self.chat.len(), None);
        }
        if self.chat_copy_hash_cache.len() < self.chat.len() {
            self.chat_copy_hash_cache.resize(self.chat.len(), None);
        }
        if self.chat_message_editors.len() < self.chat.len() {
            self.chat_message_editors.resize_with(self.chat.len(), text_editor::Content::new);
        }
        self.chat_special_text_editors.retain(|key, _| {
            let message_idx = (*key >> 32) as usize;
            message_idx < chunk_start_idx || message_idx >= chunk_end_idx
        });
        self.chat_markdown_blocks.retain(|key, _| {
            let message_idx = (*key >> 32) as usize;
            message_idx < chunk_start_idx || message_idx >= chunk_end_idx
        });
        self.chat_markdown_block_editors.retain(|key, _| {
            let message_idx = (*key >> 64) as usize;
            message_idx < chunk_start_idx || message_idx >= chunk_end_idx
        });
        self.chat_tool_text_editors.retain(|key, _| {
            let message_idx = (*key >> 64) as usize;
            message_idx < chunk_start_idx || message_idx >= chunk_end_idx
        });
        self.chat_think_editors.retain(|key, _| {
            let message_idx = (*key >> 32) as usize;
            message_idx < chunk_start_idx || message_idx >= chunk_end_idx
        });

        let mut is_chunk_fresh = true;
        for idx in chunk_start_idx..chunk_end_idx {
            let revision_key = revision_render_key(self, idx);
            (*self.chat_message_editors)[idx] = text_editor::Content::new();
            let expected_hash = crate::message_view::parse::hash_chat_content(
                &self.chat[idx].content,
            );
            let Some(render_entry) = render_cache.get(&idx) else {
                is_chunk_fresh = false;
                self.chat_render_cache.remove(&idx);
                self.chat_markdown_blocks.retain(|key, _| (*key >> 32) as usize != idx);
                self.chat_markdown_block_editors.retain(|key, _| (*key >> 64) as usize != idx);
                self.chat_explore_summary_animations.retain(|key, _| (*key >> 64) as usize != idx);
                continue;
            };

            if render_entry.content_hash != expected_hash {
                is_chunk_fresh = false;
                self.chat_render_cache.remove(&idx);
                self.chat_markdown_blocks.retain(|key, _| (*key >> 32) as usize != idx);
                self.chat_markdown_block_editors.retain(|key, _| (*key >> 64) as usize != idx);
                self.chat_explore_summary_animations.retain(|key, _| (*key >> 64) as usize != idx);
                continue;
            }

            let reuse_markdown_cache = (*self.chat_render_cache)
                .get(&idx)
                .is_some_and(|cached| cached.content_hash == render_entry.content_hash)
                && self.chat_markdown_blocks.keys().any(|key| (*key >> 32) as usize == idx);
            let mut render_entry = render_entry.clone();
            render_entry.revision_key = revision_key;
            self.chat_render_cache.insert(idx, render_entry.clone());
            sync_chat_aux_text_editors_for_message(self, idx, &render_entry, reuse_markdown_cache);
            let offset = idx - chunk_start_idx;
            (*self.chat_visible_text_cache)[idx] = visible_texts.get(offset).cloned().flatten();
            (*self.chat_copy_hash_cache)[idx] = copy_hashes.get(offset).copied().flatten();
            if let Some(Some(text)) = message_editor_texts.get(offset) {
                (*self.chat_message_editors)[idx] = text_editor::Content::with_text(text);
            }
        }

        if is_chunk_fresh {
            self.prepared_chat_ui_chunks.insert(chunk_start_idx);
        } else {
            self.prepared_chat_ui_chunks.remove(&chunk_start_idx);
        }

        self.chat_render_cache.retain(|idx, _| *idx < self.chat.len());
        self.chat_visible_text_cache.truncate(self.chat.len());
        self.chat_copy_hash_cache.truncate(self.chat.len());
        self.chat_message_editors.truncate(self.chat.len());
        self.chat_message_expanded.retain(|idx| *idx < self.chat.len());
        self.chat_message_measured_heights.retain(|idx, _| *idx < self.chat.len());
        self.refine_chat_message_estimated_heights(chunk_start_idx, chunk_end_idx);
        let (visible_start_idx, visible_end_idx) = self.visible_chat_message_window();
        self.prune_chat_ui_chunks(visible_start_idx, visible_end_idx);
        self.prune_chat_heavy_editor_caches(visible_start_idx, visible_end_idx);
    }
}

impl ChatUi<'_> {
    pub(super) fn apply_prepared_chat_ui_message(&mut self, prepared: PreparedChatUiMessage) {
        let PreparedChatUiMessage {
            message_idx,
            render_cache,
            visible_text,
            copy_hash,
            message_editor_text,
        } = prepared;
        let Some(message) = self.chat.get(message_idx) else {
            return;
        };
        let expected_hash =
            crate::message_view::parse::hash_chat_content(&message.content);
        if render_cache.content_hash != expected_hash {
            return;
        }

        if self.chat_visible_text_cache.len() < self.chat.len() {
            self.chat_visible_text_cache.resize(self.chat.len(), None);
        }
        if self.chat_copy_hash_cache.len() < self.chat.len() {
            self.chat_copy_hash_cache.resize(self.chat.len(), None);
        }
        if self.chat_message_editors.len() < self.chat.len() {
            self.chat_message_editors.resize_with(self.chat.len(), text_editor::Content::new);
        }

        self.chat_special_text_editors.retain(|key, _| (*key >> 32) as usize != message_idx);
        self.chat_tool_text_editors.retain(|key, _| (*key >> 64) as usize != message_idx);
        self.chat_think_editors.retain(|key, _| (*key >> 32) as usize != message_idx);

        let reuse_markdown_cache = (*self.chat_render_cache)
            .get(&message_idx)
            .is_some_and(|cached| cached.content_hash == render_cache.content_hash)
            && self.chat_markdown_blocks.keys().any(|key| (*key >> 32) as usize == message_idx);
        let mut render_cache = render_cache;
        render_cache.revision_key = revision_render_key(self, message_idx);
        self.chat_render_cache.insert(message_idx, render_cache.clone());
        sync_chat_aux_text_editors_for_message(
            self,
            message_idx,
            &render_cache,
            reuse_markdown_cache,
        );
        (*self.chat_visible_text_cache)[message_idx] = Some(visible_text);
        (*self.chat_copy_hash_cache)[message_idx] = Some(copy_hash);
        (*self.chat_message_editors)[message_idx] = message_editor_text
            .as_deref()
            .map(text_editor::Content::with_text)
            .unwrap_or_else(text_editor::Content::new);
        self.refine_chat_message_estimated_heights(message_idx, message_idx + 1);
    }
}
