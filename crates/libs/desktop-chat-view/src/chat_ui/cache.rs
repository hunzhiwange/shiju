//! 聊天 UI 的缓存清理与高度同步。
use super::*;

impl ChatUi<'_> {
    pub fn sync_chat_virtualizer_compat_fields(&mut self) {
        (*self.chat_message_estimated_heights) = self.chat_virtualizer.estimated_heights().to_vec();
        self.chat_height_index.set_heights(&(*self.chat_message_estimated_heights));
        self.sync_chat_scroll_compat_fields();
    }
}

impl ChatUi<'_> {
    pub fn apply_stream_tail_estimated_height(
        &mut self,
        message_idx: usize,
        estimated_height: f32,
    ) {
        desktop_session::session::active::sync_user_question_index(self.chat, self.user_question_markers, self.user_question_indexed_chat_len);
        self.chat_virtualizer.sync_len(self.chat.len(), CHAT_INITIAL_MESSAGE_HEIGHT);
        self.chat_message_measured_heights.remove(&message_idx);
        self.chat_virtualizer.apply_measured_height(message_idx, estimated_height);
        if self.chat_message_estimated_heights.len() != self.chat_virtualizer.len() {
            (*self.chat_message_estimated_heights) =
                self.chat_virtualizer.estimated_heights().to_vec();
            self.chat_height_index.set_heights(&(*self.chat_message_estimated_heights));
        } else if let Some(height) = self.chat_message_estimated_heights.get_mut(message_idx) {
            *height = estimated_height;
            self.chat_height_index.update_height(message_idx, estimated_height);
        }
        self.sync_chat_scroll_compat_fields();
    }
}

impl ChatUi<'_> {
    pub fn sync_chat_scroll_compat_fields(&mut self) {
        (*self.chat_scroll_offset_y) = self.chat_virtualizer.relative_y();
        (*self.chat_scroll_viewport_h) = self.chat_virtualizer.viewport_height_px();
    }
}

impl ChatUi<'_> {
    pub fn pin_chat_ui_chunk(&mut self, chunk_start_idx: Option<usize>) {
        (*self.pinned_chat_ui_chunk_start) = chunk_start_idx
            .filter(|_| !self.chat.is_empty())
            .map(|idx| chat_ui_chunk_start_idx(idx.min(self.chat.len().saturating_sub(1))));
    }
}

impl ChatUi<'_> {
    pub(super) fn clear_chat_ui_chunk_range(
        &mut self,
        chunk_start_idx: usize,
        chunk_end_idx: usize,
        drop_interaction_state: bool,
    ) {
        for idx in chunk_start_idx..chunk_end_idx {
            self.chat_render_cache.remove(&idx);
            if idx < self.chat_visible_text_cache.len() {
                (*self.chat_visible_text_cache)[idx] = None;
            }
            if idx < self.chat_copy_hash_cache.len() {
                (*self.chat_copy_hash_cache)[idx] = None;
            }
            if idx < self.chat_message_editors.len() {
                (*self.chat_message_editors)[idx] = text_editor::Content::new();
            }
        }

        self.chat_special_text_editors.retain(|key, _| {
            let message_idx = (*key >> 32) as usize;
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
        self.chat_think_scroll_ids.retain(|key, _| {
            let message_idx = (*key >> 32) as usize;
            message_idx < chunk_start_idx || message_idx >= chunk_end_idx
        });

        if drop_interaction_state {
            self.chat_think_expanded.retain(|key| {
                let message_idx = (*key >> 32) as usize;
                message_idx < chunk_start_idx || message_idx >= chunk_end_idx
            });
            self.chat_think_collapsed.retain(|key| {
                let message_idx = (*key >> 32) as usize;
                message_idx < chunk_start_idx || message_idx >= chunk_end_idx
            });
            self.chat_explore_expanded.retain(|key| {
                let message_idx = (*key >> 32) as usize;
                message_idx < chunk_start_idx || message_idx >= chunk_end_idx
            });
            self.chat_tool_expanded.retain(|key| {
                let message_idx = (*key >> 32) as usize;
                message_idx < chunk_start_idx || message_idx >= chunk_end_idx
            });
            if (*self.tool_detail_dialog)
                .as_ref()
                .is_some_and(|dialog| (chunk_start_idx..chunk_end_idx).contains(&dialog.msg_idx))
            {
                (*self.tool_detail_dialog) = None;
            }
        }
    }
}

impl ChatUi<'_> {
    pub fn unload_chat_ui_chunk(&mut self, chunk_start_idx: usize) {
        let (chunk_start_idx, chunk_end_idx) =
            chat_ui_chunk_bounds(self.chat.len(), chunk_start_idx);
        if chunk_start_idx >= chunk_end_idx {
            return;
        }

        self.prepared_chat_ui_chunks.remove(&chunk_start_idx);
        self.preparing_chat_ui_chunks.remove(&chunk_start_idx);
        self.dirty_preparing_chat_ui_chunks.remove(&chunk_start_idx);
        self.clear_chat_ui_chunk_range(chunk_start_idx, chunk_end_idx, false);
        self.refine_chat_message_estimated_heights(chunk_start_idx, chunk_end_idx);
    }
}

impl ChatUi<'_> {
    pub fn prune_chat_heavy_editor_caches(&mut self, start_idx: usize, end_idx: usize) {
        let schedule_input = self.chat_chunk_schedule_input(start_idx, end_idx);
        let keep_ranges =
            crate::chunk_loader::heavy_cache_keep_ranges_for_schedule(
                &schedule_input,
                CHAT_HEAVY_EDITOR_CACHE_MESSAGE_MARGIN,
            );
        let expanded_messages = self.chat_message_expanded.clone();
        let expanded_thinking = self.chat_think_expanded.clone();
        let keep_message = |message_idx: usize| {
            keep_ranges.iter().any(|(start, end)| (*start..*end).contains(&message_idx))
                || expanded_messages.contains(&message_idx)
        };

        self.chat_special_text_editors.retain(|key, _| keep_message((*key >> 32) as usize));
        self.chat_markdown_blocks.retain(|key, _| keep_message((*key >> 32) as usize));
        if (*self.chat_markdown_selection)
            .as_ref()
            .is_some_and(|selection| !keep_message(selection.message_idx))
        {
            (*self.chat_markdown_selection) = None;
        }
        self.chat_markdown_block_editors.retain(|key, _| keep_message((*key >> 64) as usize));
        self.chat_tool_text_editors.retain(|key, _| {
            let message_idx = (*key >> 64) as usize;
            keep_message(message_idx)
        });
        self.chat_think_editors.retain(|key, _| {
            let message_idx = (*key >> 32) as usize;
            keep_message(message_idx) || expanded_thinking.contains(key)
        });
    }
}

impl ChatUi<'_> {
    pub fn prune_chat_ui_chunks(&mut self, start_idx: usize, end_idx: usize) {
        let retained_chunk_starts =
            crate::chunk_loader::retained_chunk_starts_for_schedule(
                &self.chat_chunk_schedule_input(start_idx, end_idx),
            );
        let eviction_chunk_starts =
            crate::chunk_loader::eviction_chunk_starts(
                &(*self.prepared_chat_ui_chunks),
                &retained_chunk_starts,
            );

        for chunk_start_idx in eviction_chunk_starts {
            self.unload_chat_ui_chunk(chunk_start_idx);
        }
    }
}

impl ChatUi<'_> {
    pub fn pending_chat_ui_chunk_starts(
        &mut self,
        start_idx: usize,
        end_idx: usize,
        include_neighbors: bool,
    ) -> Vec<usize> {
        let schedule_input = self.chat_chunk_schedule_input(start_idx, end_idx);
        let chunk_starts =
            crate::chunk_loader::prioritized_chunk_starts(
                &schedule_input,
                include_neighbors,
            );
        let anchor_chunk_start =
            schedule_input.anchor_idx.map(chat_ui_chunk_start_idx).or_else(|| {
                chat_ui_visible_anchor_chunk_start(
                    self.chat.len(),
                    schedule_input.visible_start_idx,
                    schedule_input.visible_end_idx,
                )
            });
        if let Some(anchor_chunk_start) = anchor_chunk_start {
            (*self.last_visited_chat_ui_chunk_start) =
                Some(anchor_chunk_start);
        }
        chunk_starts
            .into_iter()
            .filter(|chunk_start_idx| {
                !self.prepared_chat_ui_chunks.contains(chunk_start_idx)
                    && !self
                        .preparing_chat_ui_chunks
                        .contains(chunk_start_idx)
            })
            .collect()
    }
}

impl ChatUi<'_> {
    pub fn mark_chat_ui_chunks_preparing(&mut self, chunk_starts: &[usize]) {
        for &chunk_start_idx in chunk_starts {
            self.preparing_chat_ui_chunks.insert(chunk_start_idx);
            self.dirty_preparing_chat_ui_chunks.remove(&chunk_start_idx);
        }
    }
}

impl ChatUi<'_> {
    pub fn invalidate_chat_ui_chunk(&mut self, chunk_start_idx: usize) {
        let (chunk_start_idx, chunk_end_idx) =
            chat_ui_chunk_bounds(self.chat.len(), chunk_start_idx);
        if chunk_start_idx >= chunk_end_idx {
            return;
        }

        self.prepared_chat_ui_chunks.remove(&chunk_start_idx);
        self.preparing_chat_ui_chunks.remove(&chunk_start_idx);
        self.dirty_preparing_chat_ui_chunks.remove(&chunk_start_idx);
        self.clear_chat_ui_chunk_range(chunk_start_idx, chunk_end_idx, true);
        self.refine_chat_message_estimated_heights(chunk_start_idx, chunk_end_idx);
    }
}

impl ChatUi<'_> {
    pub fn invalidate_chat_ui_for_message_idx(&mut self, idx: usize) {
        if idx < self.chat.len() {
            self.invalidate_chat_ui_chunk(chat_ui_chunk_start_idx(idx));
        }
    }
}

impl ChatUi<'_> {
    pub fn sync_chat_message_estimated_heights_len(&mut self) {
        desktop_session::session::active::sync_user_question_index(self.chat, self.user_question_markers, self.user_question_indexed_chat_len);
        let mut heights = self.chat_virtualizer.estimated_heights().to_vec();
        heights.truncate(self.chat.len());
        if heights.len() < self.chat.len() {
            let start_idx = heights.len();
            heights.extend(self.chat[start_idx..].iter().map(estimate_chat_message_height));
        }

        self.chat_virtualizer.set_estimated_heights(&heights);
        self.chat_message_measured_heights.retain(|idx, _| *idx < self.chat.len());
        self.sync_chat_virtualizer_compat_fields();
    }
}

impl ChatUi<'_> {
    pub fn refine_chat_message_estimated_heights(
        &mut self,
        start_idx: usize,
        end_idx: usize,
    ) {
        self.sync_chat_message_estimated_heights_len();
        let end_idx = end_idx.min(self.chat.len());
        let start_idx = start_idx.min(end_idx);

        for idx in start_idx..end_idx {
            let estimated = resolve_render_cache_height(self, idx)
                .unwrap_or_else(|| estimate_chat_message_height(&self.chat[idx]));
            let next_height = if let Some(measured) = self.chat_message_measured_heights.get(&idx) {
                *measured
            } else {
                let current = self.chat_virtualizer.estimated_height(idx).unwrap_or(estimated);
                (current * 0.35 + estimated * 0.65).max(estimated.min(current))
            };
            self.chat_virtualizer.apply_measured_height(idx, next_height);
        }
        self.sync_chat_virtualizer_compat_fields();
    }
}

impl ChatUi<'_> {
    pub fn rebuild_chat_message_estimated_heights(&mut self) {
        let heights = vec![CHAT_INITIAL_MESSAGE_HEIGHT; self.chat.len()];
        self.chat_virtualizer.set_estimated_heights(&heights);
        self.sync_chat_virtualizer_compat_fields();
    }
}

impl ChatUi<'_> {
    pub fn pending_chat_message_editors_window_chunks(
        &mut self,
        start_idx: usize,
        end_idx: usize,
    ) -> Vec<usize> {
        let pending_chunk_starts = self.pending_chat_ui_chunk_starts(start_idx, end_idx, false);
        self.prune_chat_ui_chunks(start_idx, end_idx);
        self.prune_chat_heavy_editor_caches(start_idx, end_idx);
        pending_chunk_starts
    }
}
