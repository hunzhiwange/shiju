//! 聊天 UI 的分块调度与只读投影。
use super::*;

impl ChatUiSchedule<'_> {
    pub fn protected_chat_ui_chunk_starts(&self) -> Vec<usize> {
        fn push_protected_message_idx(
            chunk_starts: &mut Vec<usize>,
            chat_len: usize,
            message_idx: usize,
        ) {
            if message_idx < chat_len {
                let chunk_start_idx = chat_ui_chunk_start_idx(message_idx);
                if !chunk_starts.contains(&chunk_start_idx) {
                    chunk_starts.push(chunk_start_idx);
                }
            }
        }

        let mut chunk_starts = Vec::new();

        if let Some(chunk_start_idx) = (*self.pinned_chat_ui_chunk_start)
            && chunk_start_idx < self.chat.len()
            && !chunk_starts.contains(&chunk_start_idx)
        {
            chunk_starts.push(chunk_start_idx);
        }

        if self.is_requesting
            && let Some(last_idx) = self.chat.len().checked_sub(1)
            && matches!(
                self.chat.get(last_idx).map(|message| message.role),
                Some(ChatRole::Assistant)
            )
        {
            push_protected_message_idx(&mut chunk_starts, self.chat.len(), last_idx);
        }

        for &message_idx in self.chat_message_expanded {
            push_protected_message_idx(&mut chunk_starts, self.chat.len(), message_idx);
        }

        for &think_key in self.chat_think_expanded {
            push_protected_message_idx(
                &mut chunk_starts,
                self.chat.len(),
                (think_key >> 32) as usize,
            );
        }

        chunk_starts
    }
    fn chat_chunk_schedule_input(
        &self,
        start_idx: usize,
        end_idx: usize,
    ) -> crate::chunk_loader::ChatChunkScheduleInput {
        let window = self.chat_virtualizer.window();
        let has_visible_window = window.visible_start_idx < window.visible_end_idx;
        let has_render_window = window.render_start_idx < window.render_end_idx;
        let visible_start_idx =
            if has_visible_window { window.visible_start_idx } else { start_idx };
        let visible_end_idx = if has_visible_window { window.visible_end_idx } else { end_idx };
        let render_start_idx = if has_render_window { window.render_start_idx } else { start_idx };
        let render_end_idx = if has_render_window { window.render_end_idx } else { end_idx };
        let anchor_idx = self.chat_virtualizer
            .anchor()
            .map(|anchor| anchor.index.min(self.chat.len().saturating_sub(1)));
        let direction_anchor_idx = anchor_idx.or_else(|| {
            chat_ui_visible_window_bounds(self.chat.len(), visible_start_idx, visible_end_idx)
                .map(|(start_idx, end_idx)| start_idx + (end_idx - start_idx - 1) / 2)
        });
        let scroll_direction = direction_anchor_idx
            .map(chat_ui_chunk_start_idx)
            .and_then(|anchor_chunk_start| {
                (*self.last_visited_chat_ui_chunk_start)
                    .map(|last_chunk_start| anchor_chunk_start.cmp(&last_chunk_start))
            })
            .map(|ordering| match ordering {
                Ordering::Greater => {
                    crate::chunk_loader::ChatChunkScrollDirection::Down
                }
                Ordering::Less => {
                    crate::chunk_loader::ChatChunkScrollDirection::Up
                }
                Ordering::Equal => crate::chunk_loader::
                    ChatChunkScrollDirection::Stationary,
            })
            .unwrap_or(
                crate::chunk_loader::ChatChunkScrollDirection::Stationary,
            );

        crate::chunk_loader::ChatChunkScheduleInput {
            chat_len: self.chat.len(),
            render_start_idx,
            render_end_idx,
            visible_start_idx,
            visible_end_idx,
            anchor_idx,
            scroll_direction,
            protected_chunk_starts: self.protected_chat_ui_chunk_starts(),
        }
    }
    pub fn visible_chat_message_window(&self) -> (usize, usize) {
        let window = self.chat_virtualizer.window();
        (window.visible_start_idx, window.visible_end_idx)
    }
    pub fn preferred_base_chat_ui_chunk_start(&self) -> usize {
        if self.chat.is_empty() {
            return 0;
        }
        let (visible_start_idx, visible_end_idx) = self.visible_chat_message_window();
        let anchor_idx = visible_start_idx.min(self.chat.len().saturating_sub(1));
        if visible_start_idx < visible_end_idx {
            chat_ui_chunk_start_idx(anchor_idx)
        } else {
            chat_ui_chunk_start_idx(self.chat.len().saturating_sub(1))
        }
    }
    pub fn collect_chat_ui_chunk_starts(
        &self,
        start_idx: usize,
        end_idx: usize,
        include_neighbors: bool,
    ) -> Vec<usize> {
        crate::chunk_loader::prioritized_chunk_starts(
            &self.chat_chunk_schedule_input(start_idx, end_idx),
            include_neighbors,
        )
    }
}
impl ChatUi<'_> {
    pub fn protected_chat_ui_chunk_starts(&self) -> Vec<usize> { self.schedule().protected_chat_ui_chunk_starts() }
}

impl ChatUi<'_> {
    pub(super) fn chat_chunk_schedule_input(
        &self,
        start_idx: usize,
        end_idx: usize,
    ) -> crate::chunk_loader::ChatChunkScheduleInput { self.schedule().chat_chunk_schedule_input(start_idx, end_idx) }
}

impl ChatUi<'_> {
    pub fn visible_chat_message_window(&self) -> (usize, usize) { self.schedule().visible_chat_message_window() }
}

impl ChatUi<'_> {
    pub fn preferred_base_chat_ui_chunk_start(&self) -> usize { self.schedule().preferred_base_chat_ui_chunk_start() }
}

impl ChatUi<'_> {
    pub fn collect_chat_ui_chunk_starts(
        &self,
        start_idx: usize,
        end_idx: usize,
        include_neighbors: bool,
    ) -> Vec<usize> { self.schedule().collect_chat_ui_chunk_starts(start_idx, end_idx, include_neighbors) }
}

impl ChatUi<'_> {
    pub(super) fn schedule(&self) -> ChatUiSchedule<'_> {
        ChatUiSchedule {
            chat: self.chat,
            chat_virtualizer: self.chat_virtualizer,
            pinned_chat_ui_chunk_start: self.pinned_chat_ui_chunk_start,
            last_visited_chat_ui_chunk_start: self.last_visited_chat_ui_chunk_start,
            chat_message_expanded: self.chat_message_expanded,
            chat_think_expanded: self.chat_think_expanded,
            is_requesting: self.is_requesting,
        }
    }
}
