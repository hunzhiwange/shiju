//! 会话聊天工作集的有界热缓存。
//!
//! 快照只保存可复用的稳定视图数据；悬浮、菜单和弹窗等瞬时交互状态始终留在
//! `App` 中，不会跨会话缓存。

use super::App;
use super::loading::session_content_signature;
use crate::app::components::chat_panel::height_index::ChatHeightIndex;
use crate::app::components::chat_panel::message_view::MarkdownBlock;
use crate::app::components::chat_panel::virtualizer::ChatVirtualizer;
use crate::app::models::{ChatMessage, ChatRenderCacheEntry, ParsedChatBlock, TokenUsage};
use crate::app::state::{ActiveSessionViewState, ChatPanelMode};
use iced::widget::text_editor;
use std::collections::{HashMap, HashSet, VecDeque};
use std::mem;
use web_time::{Duration, Instant};

pub(crate) const SESSION_VIEW_CACHE_MAX_ENTRIES: usize = 3;
pub(crate) const SESSION_VIEW_CACHE_MAX_BYTES: usize = 32 * 1024 * 1024;

/// 一个会话恢复聊天视图所需的稳定工作集。
pub(crate) struct SessionViewSnapshot {
    content_signature: u64,
    chat: Vec<ChatMessage>,
    chat_message_ids: Vec<Option<String>>,
    usage: TokenUsage,
    active_session_view_state: ActiveSessionViewState,
    chat_panel_mode: ChatPanelMode,
    chat_render_cache: HashMap<usize, ChatRenderCacheEntry>,
    chat_visible_text_cache: Vec<Option<String>>,
    chat_copy_hash_cache: Vec<Option<u64>>,
    chat_message_expanded: HashSet<usize>,
    chat_message_estimated_heights: Vec<f32>,
    chat_virtualizer: ChatVirtualizer,
    chat_height_index: ChatHeightIndex,
    chat_message_measured_heights: HashMap<usize, f32>,
    chat_message_editors: Vec<text_editor::Content>,
    chat_special_text_editors: HashMap<u64, text_editor::Content>,
    chat_markdown_blocks: HashMap<u64, Vec<MarkdownBlock>>,
    chat_markdown_block_editors: HashMap<u128, text_editor::Content>,
    chat_tool_text_editors: HashMap<u128, text_editor::Content>,
    chat_think_editors: HashMap<u64, text_editor::Content>,
    chat_think_expanded: HashSet<u64>,
    chat_think_collapsed: HashSet<u64>,
    chat_tool_file_expanded: HashSet<String>,
    chat_tool_expanded: HashSet<u64>,
    chat_explore_expanded: HashSet<u64>,
    chat_auto_scroll: bool,
    chat_scroll_id: iced::widget::Id,
    chat_scroll_offset_y: f32,
    chat_scroll_viewport_h: f32,
}

impl SessionViewSnapshot {
    /// 通过移动而非克隆，将当前聊天工作集从 `App` 中取出。
    pub(crate) fn take_from(app: &mut App) -> Self {
        let content_signature = session_content_signature(&app.chat, &app.chat_message_ids);
        Self {
            content_signature,
            chat: mem::take(&mut app.chat),
            chat_message_ids: mem::take(&mut app.chat_message_ids),
            usage: mem::take(&mut app.usage),
            active_session_view_state: mem::take(&mut app.active_session_view_state),
            chat_panel_mode: mem::take(&mut app.chat_panel_mode),
            chat_render_cache: mem::take(&mut app.chat_render_cache),
            chat_visible_text_cache: mem::take(&mut app.chat_visible_text_cache),
            chat_copy_hash_cache: mem::take(&mut app.chat_copy_hash_cache),
            chat_message_expanded: mem::take(&mut app.chat_message_expanded),
            chat_message_estimated_heights: mem::take(&mut app.chat_message_estimated_heights),
            chat_virtualizer: mem::take(&mut app.chat_virtualizer),
            chat_height_index: mem::take(&mut app.chat_height_index),
            chat_message_measured_heights: mem::take(&mut app.chat_message_measured_heights),
            chat_message_editors: mem::take(&mut app.chat_message_editors),
            chat_special_text_editors: mem::take(&mut app.chat_special_text_editors),
            chat_markdown_blocks: mem::take(&mut app.chat_markdown_blocks),
            chat_markdown_block_editors: mem::take(&mut app.chat_markdown_block_editors),
            chat_tool_text_editors: mem::take(&mut app.chat_tool_text_editors),
            chat_think_editors: mem::take(&mut app.chat_think_editors),
            chat_think_expanded: mem::take(&mut app.chat_think_expanded),
            chat_think_collapsed: mem::take(&mut app.chat_think_collapsed),
            chat_tool_file_expanded: mem::take(&mut app.chat_tool_file_expanded),
            chat_tool_expanded: mem::take(&mut app.chat_tool_expanded),
            chat_explore_expanded: mem::take(&mut app.chat_explore_expanded),
            chat_auto_scroll: mem::replace(&mut app.chat_auto_scroll, true),
            chat_scroll_id: mem::replace(&mut app.chat_scroll_id, iced::widget::Id::unique()),
            chat_scroll_offset_y: mem::replace(&mut app.chat_scroll_offset_y, 1.0),
            chat_scroll_viewport_h: mem::take(&mut app.chat_scroll_viewport_h),
        }
    }

    /// 通过移动恢复快照；会话打开流程将在后续任务中调用它。
    pub(crate) fn restore_into(self, app: &mut App) {
        app.chat = self.chat;
        app.chat_message_ids = self.chat_message_ids;
        app.usage = self.usage;
        app.active_session_view_state = self.active_session_view_state;
        app.chat_panel_mode = self.chat_panel_mode;
        app.chat_render_cache = self.chat_render_cache;
        app.chat_visible_text_cache = self.chat_visible_text_cache;
        app.chat_copy_hash_cache = self.chat_copy_hash_cache;
        app.chat_message_expanded = self.chat_message_expanded;
        app.chat_message_estimated_heights = self.chat_message_estimated_heights;
        app.chat_virtualizer = self.chat_virtualizer;
        app.chat_height_index = self.chat_height_index;
        app.chat_message_measured_heights = self.chat_message_measured_heights;
        app.chat_message_editors = self.chat_message_editors;
        app.chat_special_text_editors = self.chat_special_text_editors;
        app.chat_markdown_blocks = self.chat_markdown_blocks;
        app.chat_markdown_block_editors = self.chat_markdown_block_editors;
        app.chat_tool_text_editors = self.chat_tool_text_editors;
        app.chat_think_editors = self.chat_think_editors;
        app.chat_think_expanded = self.chat_think_expanded;
        app.chat_think_collapsed = self.chat_think_collapsed;
        app.chat_tool_file_expanded = self.chat_tool_file_expanded;
        app.chat_tool_expanded = self.chat_tool_expanded;
        app.chat_explore_expanded = self.chat_explore_expanded;
        app.chat_auto_scroll = self.chat_auto_scroll;
        app.chat_scroll_id = self.chat_scroll_id;
        app.chat_scroll_offset_y = self.chat_scroll_offset_y;
        app.chat_scroll_viewport_h = self.chat_scroll_viewport_h;
    }

    pub(crate) fn chat(&self) -> &[ChatMessage] {
        &self.chat
    }

    pub(crate) fn content_signature(&self) -> u64 {
        self.content_signature
    }

    pub(crate) fn estimated_bytes(&self) -> usize {
        let message_bytes = self
            .chat
            .iter()
            .map(|message| message.content.len() + message.model.as_deref().map_or(0, str::len));
        let render_bytes = self.chat_render_cache.values().map(render_cache_bytes);
        let visible_text_bytes =
            self.chat_visible_text_cache.iter().filter_map(Option::as_deref).map(str::len);
        let message_id_bytes =
            self.chat_message_ids.iter().filter_map(Option::as_deref).map(str::len);
        let markdown_bytes =
            self.chat_markdown_blocks.values().flatten().map(|block| block.text.len());
        let editor_bytes = self
            .chat_message_editors
            .iter()
            .chain(self.chat_special_text_editors.values())
            .chain(self.chat_markdown_block_editors.values())
            .chain(self.chat_tool_text_editors.values())
            .chain(self.chat_think_editors.values())
            .map(text_editor_bytes);
        let expanded_file_bytes = self.chat_tool_file_expanded.iter().map(String::len);
        let structural_bytes = self.chat.len() * mem::size_of::<ChatMessage>()
            + self.chat_message_ids.len() * mem::size_of::<Option<String>>()
            + self.chat_render_cache.len() * mem::size_of::<(usize, ChatRenderCacheEntry)>()
            + self.chat_visible_text_cache.len() * mem::size_of::<Option<String>>()
            + self.chat_copy_hash_cache.len() * mem::size_of::<Option<u64>>()
            + self.chat_message_estimated_heights.len() * mem::size_of::<f32>()
            + self.chat_message_measured_heights.len() * mem::size_of::<(usize, f32)>()
            + self.chat_markdown_blocks.len() * mem::size_of::<(u64, Vec<MarkdownBlock>)>();

        message_bytes
            .chain(render_bytes)
            .chain(visible_text_bytes)
            .chain(message_id_bytes)
            .chain(markdown_bytes)
            .chain(editor_bytes)
            .chain(expanded_file_bytes)
            .sum::<usize>()
            .saturating_add(self.active_session_view_state.estimated_bytes())
            .saturating_add(structural_bytes)
    }
}

fn render_cache_bytes(entry: &ChatRenderCacheEntry) -> usize {
    let block_bytes = entry.blocks.iter().map(|block| match block {
        ParsedChatBlock::Think { content, .. } | ParsedChatBlock::Text { content } => content.len(),
        ParsedChatBlock::Tool { raw } => raw.len(),
    });
    let special_text_bytes = entry.special_text_blocks.iter().map(String::len);
    let tool_text_bytes = entry.tool_card_text_blocks.iter().flatten().map(String::len);
    let explore_bytes = entry.explore_summary_text_blocks.iter().map(|(_, text)| text.len());

    entry.display_text.len()
        + entry.preview_text.len()
        + block_bytes
            .chain(special_text_bytes)
            .chain(tool_text_bytes)
            .chain(explore_bytes)
            .sum::<usize>()
}

fn text_editor_bytes(content: &text_editor::Content) -> usize {
    content.lines().map(|line| line.text.len()).sum()
}

/// 将快照插入缓存后的明确结果。
pub(crate) enum SessionViewCacheInsertResult {
    Inserted { evicted_session_ids: Vec<String> },
    RejectedOverCapacity { snapshot: SessionViewSnapshot, estimated_bytes: usize },
}

/// 热缓存发生容量回收时的原因。该枚举只描述容量条件，不携带会话或消息内容。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum SessionViewCacheEvictionReason {
    #[default]
    None,
    EntryLimit,
    ByteLimit,
    EntryAndByteLimit,
    SnapshotOverCapacity,
}

impl SessionViewCacheEvictionReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::EntryLimit => "entry_limit",
            Self::ByteLimit => "byte_limit",
            Self::EntryAndByteLimit => "entry_and_byte_limit",
            Self::SnapshotOverCapacity => "snapshot_over_capacity",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionOpenUiCommit {
    HotRestore,
    PreparedHistory,
    SameContentRefresh,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionOpenTraceSummary {
    source: &'static str,
    cache_hit: bool,
    message_count: usize,
    preparation_elapsed: Duration,
    main_thread_commit_elapsed: Duration,
    hot_restore_commits: u32,
    cold_commits: u32,
    same_content_refreshes: u32,
    changed_content_refreshes: u32,
    eviction_reason: SessionViewCacheEvictionReason,
}

impl SessionOpenTraceSummary {
    #[cfg(test)]
    pub(crate) fn source(&self) -> &'static str {
        self.source
    }

    #[cfg(test)]
    pub(crate) fn cache_hit(&self) -> bool {
        self.cache_hit
    }

    #[cfg(test)]
    pub(crate) fn ui_commit_count(&self) -> u32 {
        self.hot_restore_commits + self.cold_commits + self.changed_content_refreshes
    }

    #[cfg(test)]
    pub(crate) fn eviction_reason(&self) -> SessionViewCacheEvictionReason {
        self.eviction_reason
    }

    /// 输出一次会话打开汇总。字段只包含计数、耗时和固定枚举值。
    pub(crate) fn emit(&self) {
        let span = tracing::info_span!(
            "session_open",
            source = self.source,
            cache_hit = self.cache_hit,
            message_count = self.message_count,
            preparation_elapsed_ms = self.preparation_elapsed.as_millis() as u64,
            main_thread_commit_elapsed_ms = self.main_thread_commit_elapsed.as_millis() as u64,
            eviction_reason = self.eviction_reason.as_str(),
        );
        let _entered = span.enter();
        tracing::info!(
            target: "vw_desktop",
            hot_restore_commits = self.hot_restore_commits,
            cold_commits = self.cold_commits,
            same_content_refreshes = self.same_content_refreshes,
            changed_content_refreshes = self.changed_content_refreshes,
            "session open summary"
        );
    }
}

struct ActiveSessionOpen {
    session_id: String,
    generation: u64,
    started_at: Instant,
    message_count: usize,
    main_thread_commit_elapsed: Duration,
    hot_restore_commits: u32,
    cold_commits: u32,
    same_content_refreshes: u32,
    changed_content_refreshes: u32,
    eviction_reason: SessionViewCacheEvictionReason,
}

impl ActiveSessionOpen {
    fn record_commit(
        &mut self,
        kind: SessionOpenUiCommit,
        message_count: usize,
        commit_elapsed: Duration,
    ) {
        self.message_count = message_count;
        self.main_thread_commit_elapsed =
            self.main_thread_commit_elapsed.saturating_add(commit_elapsed);
        match kind {
            SessionOpenUiCommit::HotRestore => self.hot_restore_commits += 1,
            SessionOpenUiCommit::PreparedHistory if self.cold_commits == 0 => {
                self.cold_commits += 1;
            }
            SessionOpenUiCommit::PreparedHistory => self.changed_content_refreshes += 1,
            SessionOpenUiCommit::SameContentRefresh => self.same_content_refreshes += 1,
        }
    }

    fn summary(self) -> SessionOpenTraceSummary {
        let cache_hit = self.hot_restore_commits != 0;
        let source = if cache_hit {
            "view_cache"
        } else if self.cold_commits != 0
            && (self.same_content_refreshes != 0 || self.changed_content_refreshes != 0)
        {
            "ui_store_then_remote_history"
        } else {
            "remote_history"
        };
        SessionOpenTraceSummary {
            source,
            cache_hit,
            message_count: self.message_count,
            preparation_elapsed: self.started_at.elapsed(),
            main_thread_commit_elapsed: self.main_thread_commit_elapsed,
            hot_restore_commits: self.hot_restore_commits,
            cold_commits: self.cold_commits,
            same_content_refreshes: self.same_content_refreshes,
            changed_content_refreshes: self.changed_content_refreshes,
            eviction_reason: self.eviction_reason,
        }
    }
}

/// 确定性 LRU 会话视图缓存。
pub(crate) struct SessionViewCache {
    entries: HashMap<String, SessionViewSnapshot>,
    most_recent_last: VecDeque<String>,
    estimated_bytes: usize,
    active_open: Option<ActiveSessionOpen>,
    next_open_generation: u64,
    pending_eviction_reason: SessionViewCacheEvictionReason,
}

impl SessionViewCache {
    pub(crate) fn new() -> Self {
        Self {
            entries: HashMap::new(),
            most_recent_last: VecDeque::new(),
            estimated_bytes: 0,
            active_open: None,
            next_open_generation: 0,
            pending_eviction_reason: SessionViewCacheEvictionReason::None,
        }
    }

    /// 为一次打开请求分配世代，防止晚到的后台结果覆盖当前会话。
    pub(crate) fn begin_open(&mut self, session_id: &str) -> u64 {
        self.next_open_generation = self.next_open_generation.saturating_add(1);
        let generation = self.next_open_generation;
        self.active_open = Some(ActiveSessionOpen {
            session_id: session_id.to_owned(),
            generation,
            started_at: Instant::now(),
            message_count: 0,
            main_thread_commit_elapsed: Duration::ZERO,
            hot_restore_commits: 0,
            cold_commits: 0,
            same_content_refreshes: 0,
            changed_content_refreshes: 0,
            eviction_reason: mem::take(&mut self.pending_eviction_reason),
        });
        generation
    }

    pub(crate) fn is_current_open(&self, session_id: &str, generation: u64) -> bool {
        generation == 0
            || self.active_open.as_ref().is_some_and(|active_open| {
                active_open.session_id == session_id && active_open.generation == generation
            })
    }

    /// 记录一次主线程提交；`finalize` 时返回唯一的打开汇总事件。
    pub(crate) fn record_open_commit(
        &mut self,
        session_id: &str,
        generation: u64,
        kind: SessionOpenUiCommit,
        message_count: usize,
        commit_elapsed: Duration,
        finalize: bool,
    ) -> Option<SessionOpenTraceSummary> {
        if !self.is_current_open(session_id, generation) {
            return None;
        }
        let active_open = self.active_open.as_mut()?;
        active_open.record_commit(kind, message_count, commit_elapsed);
        if finalize { self.active_open.take().map(ActiveSessionOpen::summary) } else { None }
    }

    pub(crate) fn insert(
        &mut self,
        session_id: String,
        snapshot: SessionViewSnapshot,
    ) -> SessionViewCacheInsertResult {
        let snapshot_bytes = snapshot.estimated_bytes();
        if snapshot_bytes > SESSION_VIEW_CACHE_MAX_BYTES {
            self.pending_eviction_reason = SessionViewCacheEvictionReason::SnapshotOverCapacity;
            return SessionViewCacheInsertResult::RejectedOverCapacity {
                snapshot,
                estimated_bytes: snapshot_bytes,
            };
        }

        if let Some(previous) = self.entries.remove(&session_id) {
            self.estimated_bytes = self.estimated_bytes.saturating_sub(previous.estimated_bytes());
            self.most_recent_last.retain(|existing| existing != &session_id);
        }

        self.estimated_bytes = self.estimated_bytes.saturating_add(snapshot_bytes);
        self.most_recent_last.push_back(session_id.clone());
        self.entries.insert(session_id, snapshot);

        let mut evicted_session_ids = Vec::new();
        let mut evicted_for_entry_limit = false;
        let mut evicted_for_byte_limit = false;
        while self.entries.len() > SESSION_VIEW_CACHE_MAX_ENTRIES
            || self.estimated_bytes > SESSION_VIEW_CACHE_MAX_BYTES
        {
            evicted_for_entry_limit |= self.entries.len() > SESSION_VIEW_CACHE_MAX_ENTRIES;
            evicted_for_byte_limit |= self.estimated_bytes > SESSION_VIEW_CACHE_MAX_BYTES;
            let Some(oldest_session_id) = self.most_recent_last.pop_front() else {
                break;
            };
            if let Some(evicted) = self.entries.remove(&oldest_session_id) {
                self.estimated_bytes =
                    self.estimated_bytes.saturating_sub(evicted.estimated_bytes());
                evicted_session_ids.push(oldest_session_id);
            }
        }

        self.pending_eviction_reason = if evicted_session_ids.is_empty() {
            SessionViewCacheEvictionReason::None
        } else {
            match (evicted_for_entry_limit, evicted_for_byte_limit) {
                (true, true) => SessionViewCacheEvictionReason::EntryAndByteLimit,
                (true, false) => SessionViewCacheEvictionReason::EntryLimit,
                (false, true) => SessionViewCacheEvictionReason::ByteLimit,
                (false, false) => SessionViewCacheEvictionReason::EntryLimit,
            }
        };

        SessionViewCacheInsertResult::Inserted { evicted_session_ids }
    }

    #[cfg(test)]
    pub(crate) fn get(&mut self, session_id: &str) -> Option<&SessionViewSnapshot> {
        if !self.entries.contains_key(session_id) {
            return None;
        }
        self.most_recent_last.retain(|existing| existing != session_id);
        self.most_recent_last.push_back(session_id.to_owned());
        self.entries.get(session_id)
    }

    pub(crate) fn take(&mut self, session_id: &str) -> Option<SessionViewSnapshot> {
        let snapshot = self.entries.remove(session_id)?;
        self.estimated_bytes = self.estimated_bytes.saturating_sub(snapshot.estimated_bytes());
        self.most_recent_last.retain(|existing| existing != session_id);
        Some(snapshot)
    }

    /// 仅在写入内容与热快照不同的时候使其失效，保留相同内容的工作集。
    pub(crate) fn invalidate_if_content_changed(
        &mut self,
        session_id: &str,
        content_signature: u64,
    ) -> bool {
        if self
            .entries
            .get(session_id)
            .is_none_or(|snapshot| snapshot.content_signature() == content_signature)
        {
            return false;
        }
        self.take(session_id).is_some()
    }

    pub(crate) fn remove(&mut self, session_id: &str) -> Option<SessionViewSnapshot> {
        self.take(session_id)
    }

    #[cfg(test)]
    pub(crate) fn contains(&self, session_id: &str) -> bool {
        self.entries.contains_key(session_id)
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }
}

impl Default for SessionViewCache {
    fn default() -> Self {
        Self::new()
    }
}
