//! 桌面聊天字段借用与领域更新委派。
use super::App;
pub use desktop_chat_view::chat_ui::{PreparedChatUiChunk, PreparedChatUiMessage, PreparedChatUiPhase,
    CHAT_UI_CHUNK_SIZE, prepare_chat_ui_chunk_phase, prepare_chat_ui_message_phase,
    chat_ui_chunk_start_idx, chat_ui_chunk_bounds};

impl App {
    fn chat_ui(&mut self) -> desktop_chat_view::chat_ui::ChatUi<'_> {
        let is_dark = crate::app::components::chat_panel::utils::is_dark_theme(&self.theme());
        let is_requesting = self.current_session_runtime_ref().is_some_and(|r| r.request_lifecycle.is_requesting);
        desktop_chat_view::chat_ui::ChatUi {
            chat: &self.chat,
            chat_copy_hash_cache: &mut self.chat_copy_hash_cache,
            chat_explore_expanded: &mut self.chat_explore_expanded,
            chat_explore_summary_animations: &mut self.chat_explore_summary_animations,
            chat_height_index: &mut self.chat_height_index,
            chat_markdown_block_editors: &mut self.chat_markdown_block_editors,
            chat_markdown_blocks: &mut self.chat_markdown_blocks,
            chat_markdown_selection: &mut self.chat_markdown_selection,
            chat_message_editors: &mut self.chat_message_editors,
            chat_message_estimated_heights: &mut self.chat_message_estimated_heights,
            chat_message_expanded: &mut self.chat_message_expanded,
            chat_message_measured_heights: &mut self.chat_message_measured_heights,
            chat_render_cache: &mut self.chat_render_cache,
            chat_reset_menu_idx: &mut self.chat_reset_menu_idx,
            chat_scroll_offset_y: &mut self.chat_scroll_offset_y,
            chat_scroll_viewport_h: &mut self.chat_scroll_viewport_h,
            chat_special_text_editors: &mut self.chat_special_text_editors,
            chat_think_collapsed: &mut self.chat_think_collapsed,
            chat_think_editors: &mut self.chat_think_editors,
            chat_think_expanded: &mut self.chat_think_expanded,
            chat_think_hovered_idx: &mut self.chat_think_hovered_idx,
            chat_think_scroll_ids: &mut self.chat_think_scroll_ids,
            chat_tool_expanded: &mut self.chat_tool_expanded,
            chat_tool_file_expanded: &mut self.chat_tool_file_expanded,
            chat_tool_file_hovered: &mut self.chat_tool_file_hovered,
            chat_tool_hovered_idx: &mut self.chat_tool_hovered_idx,
            chat_tool_text_editors: &mut self.chat_tool_text_editors,
            chat_virtualizer: &mut self.chat_virtualizer,
            chat_visible_text_cache: &mut self.chat_visible_text_cache,
            tool_detail_dialog: &mut self.tool_detail_dialog,
            message_meta_texts: &mut self.active_session_view_state.message_meta_texts,
            user_question_markers: &mut self.active_session_view_state.user_question_markers,
            user_question_indexed_chat_len: &mut self.active_session_view_state.user_question_indexed_chat_len,
            prepared_chat_ui_chunks: &mut self.active_session_view_state.prepared_chat_ui_chunks,
            preparing_chat_ui_chunks: &mut self.active_session_view_state.preparing_chat_ui_chunks,
            dirty_preparing_chat_ui_chunks: &mut self.active_session_view_state.dirty_preparing_chat_ui_chunks,
            last_visited_chat_ui_chunk_start: &mut self.active_session_view_state.last_visited_chat_ui_chunk_start,
            pinned_chat_ui_chunk_start: &mut self.active_session_view_state.pinned_chat_ui_chunk_start,
            desktop_locale: self.desktop_locale,
            dialogue_flow_show_reasoning_summary: self.dialogue_flow_show_reasoning_summary,
            timeline: self.paged_chat_timeline.as_ref(),
            window_width: self.window_size.0,
            is_dark, is_requesting,
        }
    }
    fn chat_ui_schedule(&self) -> desktop_chat_view::chat_ui::ChatUiSchedule<'_> {
        desktop_chat_view::chat_ui::ChatUiSchedule {
            chat: &self.chat,
            chat_virtualizer: &self.chat_virtualizer,
            pinned_chat_ui_chunk_start: &self.active_session_view_state.pinned_chat_ui_chunk_start,
            last_visited_chat_ui_chunk_start: &self.active_session_view_state.last_visited_chat_ui_chunk_start,
            chat_message_expanded: &self.chat_message_expanded,
            chat_think_expanded: &self.chat_think_expanded,
            is_requesting: self.current_session_runtime_ref().is_some_and(|r| r.request_lifecycle.is_requesting),
        }
    }
    pub(crate) fn sync_completed_stream_text(&mut self, msg_idx: usize) { self.chat_ui().sync_completed_stream_text(msg_idx) }
    pub(crate) fn finalize_stream_text(&mut self) { self.chat_ui().finalize_stream_text() }
    pub(crate) fn sync_chat_virtualizer_compat_fields(&mut self) { self.chat_ui().sync_chat_virtualizer_compat_fields() }
    pub(crate) fn apply_stream_tail_estimated_height(
        &mut self,
        message_idx: usize,
        estimated_height: f32,
    ) { self.chat_ui().apply_stream_tail_estimated_height(message_idx, estimated_height) }
    pub(crate) fn sync_chat_scroll_compat_fields(&mut self) { self.chat_ui().sync_chat_scroll_compat_fields() }
    pub(crate) fn protected_chat_ui_chunk_starts(&self) -> Vec<usize> { self.chat_ui_schedule().protected_chat_ui_chunk_starts() }
    pub(crate) fn pin_chat_ui_chunk(&mut self, chunk_start_idx: Option<usize>) { self.chat_ui().pin_chat_ui_chunk(chunk_start_idx) }
    pub(crate) fn unload_chat_ui_chunk(&mut self, chunk_start_idx: usize) { self.chat_ui().unload_chat_ui_chunk(chunk_start_idx) }
    pub(crate) fn visible_chat_message_window(&self) -> (usize, usize) { self.chat_ui_schedule().visible_chat_message_window() }
    pub(crate) fn prune_chat_heavy_editor_caches(&mut self, start_idx: usize, end_idx: usize) { self.chat_ui().prune_chat_heavy_editor_caches(start_idx, end_idx) }
    pub(crate) fn prune_chat_ui_chunks(&mut self, start_idx: usize, end_idx: usize) { self.chat_ui().prune_chat_ui_chunks(start_idx, end_idx) }
    pub(crate) fn preferred_base_chat_ui_chunk_start(&self) -> usize { self.chat_ui_schedule().preferred_base_chat_ui_chunk_start() }
    pub(crate) fn collect_chat_ui_chunk_starts(
        &self,
        start_idx: usize,
        end_idx: usize,
        include_neighbors: bool,
    ) -> Vec<usize> { self.chat_ui_schedule().collect_chat_ui_chunk_starts(start_idx, end_idx, include_neighbors) }
    pub(crate) fn pending_chat_ui_chunk_starts(
        &mut self,
        start_idx: usize,
        end_idx: usize,
        include_neighbors: bool,
    ) -> Vec<usize> { self.chat_ui().pending_chat_ui_chunk_starts(start_idx, end_idx, include_neighbors) }
    pub(crate) fn mark_chat_ui_chunks_preparing(&mut self, chunk_starts: &[usize]) { self.chat_ui().mark_chat_ui_chunks_preparing(chunk_starts) }
    pub(crate) fn invalidate_chat_ui_chunk(&mut self, chunk_start_idx: usize) { self.chat_ui().invalidate_chat_ui_chunk(chunk_start_idx) }
    pub(crate) fn invalidate_chat_ui_for_message_idx(&mut self, idx: usize) { self.chat_ui().invalidate_chat_ui_for_message_idx(idx) }
    pub(crate) fn sync_chat_message_estimated_heights_len(&mut self) { self.chat_ui().sync_chat_message_estimated_heights_len() }
    pub(crate) fn refine_chat_message_estimated_heights(
        &mut self,
        start_idx: usize,
        end_idx: usize,
    ) { self.chat_ui().refine_chat_message_estimated_heights(start_idx, end_idx) }
    pub(crate) fn rebuild_chat_message_estimated_heights(&mut self) { self.chat_ui().rebuild_chat_message_estimated_heights() }
    pub fn invalidate_chat_ui_state(&mut self) { self.chat_ui().invalidate_chat_ui_state() }
    pub fn apply_prepared_chat_ui_phase(&mut self, phase: PreparedChatUiPhase) { self.chat_ui().apply_prepared_chat_ui_phase(phase) }
    pub(crate) fn pending_chat_message_editors_window_chunks(
        &mut self,
        start_idx: usize,
        end_idx: usize,
    ) -> Vec<usize> { self.chat_ui().pending_chat_message_editors_window_chunks(start_idx, end_idx) }
}
