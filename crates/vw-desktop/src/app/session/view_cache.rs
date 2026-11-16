//! Desktop field mapping for the owned render snapshot.
use super::App;
use super::loading::session_content_signature;
use crate::app::models::ChatMessage;
use crate::app::state::{ActiveSessionViewState, ChatPanelMode};
use std::mem;
pub(crate) use desktop_session::session::view_cache::{SessionOpenUiCommit, SessionViewCacheInsertResult};
pub(crate) type SessionViewCache = desktop_session::session::view_cache::SessionViewCache<SessionViewSnapshot>;
pub(crate) struct SessionViewSnapshot(desktop_chat_view::view_snapshot::SessionViewSnapshot<ActiveSessionViewState, ChatPanelMode>);
impl SessionViewSnapshot {
    /// 通过移动而非克隆，将当前聊天工作集从 `App` 中取出。
    pub(crate) fn take_from(app: &mut App) -> Self {
        app.chat_interaction.chat_markdown_selection = None;
        let content_signature = session_content_signature(&app.chat_view.chat, &app.chat_view.chat_message_ids);
        Self(desktop_chat_view::view_snapshot::SessionViewSnapshot {
            content_signature,
            chat: mem::take(&mut app.chat_view.chat),
            chat_message_ids: mem::take(&mut app.chat_view.chat_message_ids),
            usage: mem::take(&mut app.chat_view.usage),
            active_session_view_state: mem::take(&mut app.chat_view.active_session_view_state),
            chat_panel_mode: mem::take(&mut app.chat_view.chat_panel_mode),
            chat_render_cache: mem::take(&mut app.chat_view.chat_render_cache),
            chat_visible_text_cache: mem::take(&mut app.chat_view.chat_visible_text_cache),
            chat_copy_hash_cache: mem::take(&mut app.chat_view.chat_copy_hash_cache),
            chat_message_expanded: mem::take(&mut app.chat_view.chat_message_expanded),
            chat_message_estimated_heights: mem::take(&mut app.chat_view.chat_message_estimated_heights),
            chat_virtualizer: mem::take(&mut app.chat_view.chat_virtualizer),
            chat_height_index: mem::take(&mut app.chat_view.chat_height_index),
            chat_message_measured_heights: mem::take(&mut app.chat_view.chat_message_measured_heights),
            chat_message_editors: mem::take(&mut app.chat_view.chat_message_editors),
            chat_special_text_editors: mem::take(&mut app.chat_view.chat_special_text_editors),
            chat_markdown_blocks: mem::take(&mut app.chat_view.chat_markdown_blocks),
            chat_markdown_block_editors: mem::take(&mut app.chat_view.chat_markdown_block_editors),
            chat_tool_text_editors: mem::take(&mut app.chat_view.chat_tool_text_editors),
            chat_think_editors: mem::take(&mut app.chat_view.chat_think_editors),
            chat_think_expanded: mem::take(&mut app.chat_view.chat_think_expanded),
            chat_think_collapsed: mem::take(&mut app.chat_view.chat_think_collapsed),
            chat_tool_file_expanded: mem::take(&mut app.chat_view.chat_tool_file_expanded),
            chat_tool_expanded: mem::take(&mut app.chat_view.chat_tool_expanded),
            chat_explore_expanded: mem::take(&mut app.chat_view.chat_explore_expanded),
            chat_auto_scroll: mem::replace(&mut app.chat_view.chat_auto_scroll, true),
            chat_scroll_id: mem::replace(&mut app.chat_view.chat_scroll_id, iced::widget::Id::unique()),
            chat_scroll_offset_y: mem::replace(&mut app.chat_view.chat_scroll_offset_y, 1.0),
            chat_scroll_viewport_h: mem::take(&mut app.chat_view.chat_scroll_viewport_h),
        })
    }

    /// 通过移动恢复快照；会话打开流程将在后续任务中调用它。
    pub(crate) fn restore_into(self, app: &mut App) {
        let Self(snapshot) = self;
        app.chat_view.chat = snapshot.chat;
        app.chat_view.chat_message_ids = snapshot.chat_message_ids;
        app.chat_view.usage = snapshot.usage;
        app.chat_view.active_session_view_state = snapshot.active_session_view_state;
        app.chat_view.chat_panel_mode = snapshot.chat_panel_mode;
        app.chat_view.chat_render_cache = snapshot.chat_render_cache;
        app.chat_view.chat_visible_text_cache = snapshot.chat_visible_text_cache;
        app.chat_view.chat_copy_hash_cache = snapshot.chat_copy_hash_cache;
        app.chat_view.chat_message_expanded = snapshot.chat_message_expanded;
        app.chat_view.chat_message_estimated_heights = snapshot.chat_message_estimated_heights;
        app.chat_view.chat_virtualizer = snapshot.chat_virtualizer;
        app.chat_view.chat_height_index = snapshot.chat_height_index;
        app.chat_view.chat_message_measured_heights = snapshot.chat_message_measured_heights;
        app.chat_view.chat_message_editors = snapshot.chat_message_editors;
        app.chat_view.chat_special_text_editors = snapshot.chat_special_text_editors;
        app.chat_view.chat_markdown_blocks = snapshot.chat_markdown_blocks;
        app.chat_view.chat_markdown_block_editors = snapshot.chat_markdown_block_editors;
        app.chat_view.chat_tool_text_editors = snapshot.chat_tool_text_editors;
        app.chat_view.chat_think_editors = snapshot.chat_think_editors;
        app.chat_view.chat_think_expanded = snapshot.chat_think_expanded;
        app.chat_view.chat_think_collapsed = snapshot.chat_think_collapsed;
        app.chat_view.chat_tool_file_expanded = snapshot.chat_tool_file_expanded;
        app.chat_view.chat_tool_expanded = snapshot.chat_tool_expanded;
        app.chat_view.chat_explore_expanded = snapshot.chat_explore_expanded;
        app.chat_view.chat_auto_scroll = snapshot.chat_auto_scroll;
        app.chat_view.chat_scroll_id = snapshot.chat_scroll_id;
        app.chat_view.chat_scroll_offset_y = snapshot.chat_scroll_offset_y;
        app.chat_view.chat_scroll_viewport_h = snapshot.chat_scroll_viewport_h;
    }

    pub(crate) fn chat(&self) -> &[ChatMessage] { self.0.chat() }
    pub(crate) fn content_signature(&self) -> u64 { self.0.content_signature() }
    pub(crate) fn estimated_bytes(&self) -> usize { self.0.estimated_bytes(self.0.active_session_view_state.estimated_bytes()) }
}
impl desktop_session::session::view_cache::ViewSnapshot for SessionViewSnapshot {
    fn content_signature(&self) -> u64 { self.content_signature() }
    fn estimated_bytes(&self) -> usize { self.estimated_bytes() }
}
