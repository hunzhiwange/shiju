//! 维护聊天会话的加载、运行态和 UI 分块派生逻辑。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use super::{App, ChatMessage, ChatRenderCacheEntry, ChatRole};
use crate::app::models::ParsedChatBlock;
use iced::widget::text_editor;
use std::cmp::Ordering;
use std::collections::HashMap;

#[derive(Debug, Clone)]
/// 表示 PreparedChatUiChunk 相关的应用状态或派生数据。
pub struct PreparedChatUiChunk {
    pub chunk_start_idx: usize,
    pub chunk_end_idx: usize,
    pub render_cache: HashMap<usize, ChatRenderCacheEntry>,
    pub visible_texts: Vec<Option<String>>,
    pub copy_hashes: Vec<Option<u64>>,
    pub message_editor_texts: Vec<Option<String>>,
}

#[derive(Debug, Clone)]
pub struct PreparedChatUiMessage {
    pub message_idx: usize,
    pub render_cache: ChatRenderCacheEntry,
    pub visible_text: String,
    pub copy_hash: u64,
    pub message_editor_text: Option<String>,
}

#[derive(Debug, Clone)]
/// 描述 PreparedChatUiPhase 支持的离散状态或消息分支。
pub enum PreparedChatUiPhase {
    Base(PreparedChatUiChunk),
    Chunk(PreparedChatUiChunk),
    Message(PreparedChatUiMessage),
}

/// CHAT_UI_CHUNK_SIZE 使用的固定配置值。
pub const CHAT_UI_CHUNK_SIZE: usize = 32;
const CHAT_HEAVY_EDITOR_CACHE_MESSAGE_MARGIN: usize = 24;
pub(crate) const CHAT_INITIAL_MESSAGE_HEIGHT: f32 = 120.0;

#[derive(Debug, Clone, Default)]
struct ParsedChatUiArtifacts {
    render_cache: HashMap<usize, ChatRenderCacheEntry>,
    visible_texts: Vec<Option<String>>,
    copy_hashes: Vec<Option<u64>>,
    message_editor_texts: Vec<Option<String>>,
}

fn parse_chat_ui_artifacts(
    chat: &[ChatMessage],
    show_reasoning_summary: bool,
) -> ParsedChatUiArtifacts {
    let mut parsed = ParsedChatUiArtifacts {
        visible_texts: vec![None; chat.len()],
        copy_hashes: vec![None; chat.len()],
        message_editor_texts: vec![None; chat.len()],
        ..ParsedChatUiArtifacts::default()
    };

    for (msg_idx, message) in chat.iter().enumerate() {
        let visible = match message.role {
            ChatRole::Assistant => crate::app::ui::chat::split_think(&message.content).1,
            ChatRole::Tool | ChatRole::User | ChatRole::System => message.content.clone(),
        };
        let copy_hash =
            crate::app::components::chat_panel::message_view::hash_chat_content(&visible);
        let entry = crate::app::components::chat_panel::message_view::build_render_cache_entry(
            &message.content,
            &visible,
            copy_hash,
            show_reasoning_summary,
        );

        parsed.visible_texts[msg_idx] = Some(visible.clone());
        parsed.copy_hashes[msg_idx] = Some(copy_hash);
        let editor_text =
            crate::app::components::chat_panel::selectable_text::visible_editor_text_for_message(
                message.role,
                &message.content,
                &visible,
                &entry.display_text,
            );
        if crate::app::components::chat_panel::selectable_text::read_only_editor_content(
            &editor_text,
        )
        .is_some()
        {
            parsed.message_editor_texts[msg_idx] = Some(editor_text);
        }

        parsed.render_cache.insert(msg_idx, entry);
    }

    parsed
}

fn resolve_render_cache_height(app: &App, idx: usize) -> Option<f32> {
    let message = app.chat.get(idx)?;
    let cache_entry = app.chat_render_cache.get(&idx)?;
    if cache_entry.content_hash
        != crate::app::components::chat_panel::message_view::hash_chat_content(&message.content)
    {
        return None;
    }
    Some(cache_entry.estimated_expanded_height)
}

fn estimate_chat_message_height(message: &ChatMessage) -> f32 {
    crate::app::components::chat_panel::message_view::estimate_message_height_rough(
        &message.content,
    )
}

/// 执行 chat_ui_chunk_start_idx 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn chat_ui_chunk_start_idx(idx: usize) -> usize {
    (idx / CHAT_UI_CHUNK_SIZE) * CHAT_UI_CHUNK_SIZE
}

/// 执行 chat_ui_chunk_bounds 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn chat_ui_chunk_bounds(chat_len: usize, chunk_start_idx: usize) -> (usize, usize) {
    let chunk_start_idx = chunk_start_idx.min(chat_len);
    let chunk_end_idx = chunk_start_idx.saturating_add(CHAT_UI_CHUNK_SIZE).min(chat_len);
    (chunk_start_idx, chunk_end_idx)
}

fn chat_ui_visible_window_bounds(
    chat_len: usize,
    start_idx: usize,
    end_idx: usize,
) -> Option<(usize, usize)> {
    if chat_len == 0 {
        return None;
    }

    let visible_start_idx = start_idx.min(chat_len.saturating_sub(1));
    let visible_end_idx = end_idx.max(visible_start_idx.saturating_add(1)).min(chat_len);
    Some((visible_start_idx, visible_end_idx))
}

fn chat_ui_visible_anchor_chunk_start(
    chat_len: usize,
    start_idx: usize,
    end_idx: usize,
) -> Option<usize> {
    let (visible_start_idx, visible_end_idx) =
        chat_ui_visible_window_bounds(chat_len, start_idx, end_idx)?;
    let anchor_idx = visible_start_idx + (visible_end_idx - visible_start_idx - 1) / 2;
    Some(chat_ui_chunk_start_idx(anchor_idx))
}

#[cfg(test)]
fn sort_chunk_starts_by_anchor_distance(chunk_starts: &mut [usize], anchor_chunk_start: usize) {
    chunk_starts.sort_by_key(|chunk_start_idx| {
        (chunk_start_idx.abs_diff(anchor_chunk_start), *chunk_start_idx)
    });
}

/// 执行 prioritize_chat_ui_chunk_starts 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
#[cfg(test)]
pub(super) fn prioritize_chat_ui_chunk_starts(
    chat_len: usize,
    start_idx: usize,
    end_idx: usize,
    include_neighbors: bool,
    last_visited_chunk_start: Option<usize>,
    base_chunk_start: usize,
) -> (Vec<usize>, Option<usize>) {
    let Some((visible_start_idx, visible_end_idx)) =
        chat_ui_visible_window_bounds(chat_len, start_idx, end_idx)
    else {
        return (Vec::new(), None);
    };

    let anchor_chunk_start =
        chat_ui_visible_anchor_chunk_start(chat_len, visible_start_idx, visible_end_idx)
            .unwrap_or(base_chunk_start.min(chat_len.saturating_sub(1)));
    let direction = last_visited_chunk_start
        .map(|last_chunk_start| anchor_chunk_start.cmp(&last_chunk_start))
        .unwrap_or(Ordering::Equal);

    let mut visible_chunk_starts = Vec::new();
    let mut chunk_start_idx = chat_ui_chunk_start_idx(visible_start_idx);
    while chunk_start_idx < visible_end_idx {
        visible_chunk_starts.push(chunk_start_idx);
        chunk_start_idx = chunk_start_idx.saturating_add(CHAT_UI_CHUNK_SIZE);
    }

    match direction {
        Ordering::Greater => visible_chunk_starts.sort_unstable_by(|left, right| right.cmp(left)),
        Ordering::Less => visible_chunk_starts.sort_unstable(),
        Ordering::Equal => {
            sort_chunk_starts_by_anchor_distance(&mut visible_chunk_starts, anchor_chunk_start)
        }
    }

    let first_visible_chunk_start = chat_ui_chunk_start_idx(visible_start_idx);
    let last_visible_chunk_start = chat_ui_chunk_start_idx(visible_end_idx.saturating_sub(1));
    let previous_chunk_start = first_visible_chunk_start.checked_sub(CHAT_UI_CHUNK_SIZE);
    let next_chunk_start =
        last_visible_chunk_start.saturating_add(CHAT_UI_CHUNK_SIZE).min(chat_len);

    let mut prioritized_chunk_starts = Vec::new();
    let mut push_chunk = |chunk_start_idx: usize| {
        if chunk_start_idx < chat_len && !prioritized_chunk_starts.contains(&chunk_start_idx) {
            prioritized_chunk_starts.push(chunk_start_idx);
        }
    };

    if last_visited_chunk_start.is_none() || base_chunk_start == anchor_chunk_start {
        push_chunk(base_chunk_start);
    }

    for chunk_start_idx in visible_chunk_starts {
        push_chunk(chunk_start_idx);
    }

    if include_neighbors {
        match direction {
            Ordering::Greater => {
                if next_chunk_start < chat_len {
                    push_chunk(next_chunk_start);
                }
                if let Some(previous_chunk_start) = previous_chunk_start {
                    push_chunk(previous_chunk_start);
                }
            }
            Ordering::Less => {
                if let Some(previous_chunk_start) = previous_chunk_start {
                    push_chunk(previous_chunk_start);
                }
                if next_chunk_start < chat_len {
                    push_chunk(next_chunk_start);
                }
            }
            Ordering::Equal => {
                let mut neighbor_chunk_starts = Vec::new();
                if let Some(previous_chunk_start) = previous_chunk_start {
                    neighbor_chunk_starts.push(previous_chunk_start);
                }
                if next_chunk_start < chat_len {
                    neighbor_chunk_starts.push(next_chunk_start);
                }
                sort_chunk_starts_by_anchor_distance(
                    &mut neighbor_chunk_starts,
                    anchor_chunk_start,
                );
                for chunk_start_idx in neighbor_chunk_starts {
                    push_chunk(chunk_start_idx);
                }
            }
        }
    }

    push_chunk(base_chunk_start);

    (prioritized_chunk_starts, Some(anchor_chunk_start))
}

fn build_prepared_chat_ui_chunk(
    chat_window: &[ChatMessage],
    chunk_start_idx: usize,
    show_reasoning_summary: bool,
) -> PreparedChatUiChunk {
    let parsed = parse_chat_ui_artifacts(chat_window, show_reasoning_summary);
    PreparedChatUiChunk {
        chunk_start_idx,
        chunk_end_idx: chunk_start_idx + chat_window.len(),
        render_cache: parsed
            .render_cache
            .into_iter()
            .map(|(idx, entry)| (idx + chunk_start_idx, entry))
            .collect(),
        visible_texts: parsed.visible_texts,
        copy_hashes: parsed.copy_hashes,
        message_editor_texts: parsed.message_editor_texts,
    }
}

fn read_only_chat_editor_content(text: &str) -> Option<text_editor::Content> {
    crate::app::components::chat_panel::selectable_text::read_only_editor_content(text)
}

/// 执行 explore_summary_animation_key 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(super) fn explore_summary_animation_key(msg_idx: usize, group_idx: usize) -> u128 {
    ((msg_idx as u128) << 64) | (group_idx as u128)
}

fn sync_chat_explore_summary_animations_for_message(
    app: &mut App,
    msg_idx: usize,
    render_entry: &ChatRenderCacheEntry,
) {
    let now_ms = crate::app::time::now_ms();

    for (group_idx, (_, summary_text)) in
        render_entry.explore_summary_text_blocks.iter().enumerate()
    {
        let key = explore_summary_animation_key(msg_idx, group_idx);
        match app.chat_explore_summary_animations.entry(key) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                let state = entry.get_mut();
                if state.current_summary_text != *summary_text {
                    state.previous_summary_text = state.current_summary_text.clone();
                    state.current_summary_text = summary_text.clone();
                    state.changed_at_ms = Some(now_ms);
                }
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(crate::app::state::ExploreSummaryAnimationState {
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

/// 解析一段助手正文为 Markdown 块，并为每个块建立可划选只读编辑器。
///
/// 块结构存入 `chat_markdown_blocks`，对应编辑器存入 `chat_markdown_block_editors`，
/// 二者键方案分别按消息+文本块索引、消息+文本块+块索引。
fn sync_markdown_blocks_for_text(app: &mut App, msg_idx: usize, text_idx: usize, text: &str) {
    use crate::app::components::chat_panel::message_view;

    let blocks = message_view::parse_markdown_blocks(text);
    for (block_idx, block) in blocks.iter().enumerate() {
        if let Some(editor) = read_only_chat_editor_content(&block.text) {
            let key = message_view::markdown_block_key(msg_idx, text_idx, block_idx);
            app.chat_markdown_block_editors.insert(key, editor);
        }
    }
    let block_key = ((msg_idx as u64) << 32) | (text_idx as u64);
    app.chat_markdown_blocks.insert(block_key, blocks);
}

fn sync_chat_aux_text_editors_for_message(
    app: &mut App,
    msg_idx: usize,
    render_entry: &ChatRenderCacheEntry,
) {
    sync_chat_explore_summary_animations_for_message(app, msg_idx, render_entry);

    let is_assistant_message =
        matches!(app.chat.get(msg_idx).map(|message| message.role), Some(ChatRole::Assistant));

    app.chat_markdown_blocks.retain(|key, _| (*key >> 32) as usize != msg_idx);
    app.chat_markdown_block_editors.retain(|key, _| (*key >> 64) as usize != msg_idx);

    for (text_idx, text) in render_entry.special_text_blocks.iter().enumerate() {
        let key = ((msg_idx as u64) << 32) | (text_idx as u64);
        if let Some(editor) = read_only_chat_editor_content(text) {
            app.chat_special_text_editors.insert(key, editor);
        }
        if is_assistant_message {
            sync_markdown_blocks_for_text(app, msg_idx, text_idx, text);
        }
    }

    // 非特殊助手消息整体走 display_text，按文本块索引 0 缓存 Markdown 块。
    if is_assistant_message
        && !render_entry.has_special_blocks
        && !render_entry.display_text.trim().is_empty()
    {
        sync_markdown_blocks_for_text(app, msg_idx, 0, &render_entry.display_text);
    }

    for (tool_idx, texts) in render_entry.tool_card_text_blocks.iter().enumerate() {
        for (text_idx, text) in texts.iter().enumerate() {
            if let Some(editor) = read_only_chat_editor_content(text) {
                let key = crate::app::components::chat_panel::tool_text_support::tool_text_key(
                    msg_idx, tool_idx, text_idx,
                );
                app.chat_tool_text_editors.insert(key, editor);
            }
        }
    }

    for (group_tool_idx, summary_text) in &render_entry.explore_summary_text_blocks {
        if let Some(editor) = read_only_chat_editor_content(summary_text) {
            let key = crate::app::components::chat_panel::tool_text_support::tool_text_key(
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
                crate::app::components::chat_panel::utils::normalize_display_text(content.trim())
                    .into_owned();
            if let Some(editor) = read_only_chat_editor_content(&normalized) {
                let key = ((msg_idx as u64) << 32) | (think_idx as u64);
                app.chat_think_editors.insert(key, editor);
            }
            think_idx += 1;
        }
    }
}

/// 执行 prepare_chat_ui_chunk_phase 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn prepare_chat_ui_chunk_phase(
    chat_window: &[ChatMessage],
    chunk_start_idx: usize,
    is_base: bool,
    show_reasoning_summary: bool,
) -> PreparedChatUiPhase {
    let chunk = build_prepared_chat_ui_chunk(chat_window, chunk_start_idx, show_reasoning_summary);
    if is_base { PreparedChatUiPhase::Base(chunk) } else { PreparedChatUiPhase::Chunk(chunk) }
}

pub fn prepare_chat_ui_message_phase(
    message: &ChatMessage,
    message_idx: usize,
    show_reasoning_summary: bool,
) -> PreparedChatUiPhase {
    let mut parsed = parse_chat_ui_artifacts(std::slice::from_ref(message), show_reasoning_summary);
    PreparedChatUiPhase::Message(PreparedChatUiMessage {
        message_idx,
        render_cache: parsed.render_cache.remove(&0).unwrap_or_default(),
        visible_text: parsed.visible_texts.pop().flatten().unwrap_or_default(),
        copy_hash: parsed.copy_hashes.pop().flatten().unwrap_or_default(),
        message_editor_text: parsed.message_editor_texts.pop().flatten(),
    })
}

impl App {
    pub(crate) fn sync_chat_virtualizer_compat_fields(&mut self) {
        self.chat_message_estimated_heights = self.chat_virtualizer.estimated_heights().to_vec();
        self.chat_height_index.set_heights(&self.chat_message_estimated_heights);
        self.sync_chat_scroll_compat_fields();
    }

    pub(crate) fn apply_stream_tail_estimated_height(
        &mut self,
        message_idx: usize,
        estimated_height: f32,
    ) {
        self.sync_active_session_user_question_index();
        self.chat_virtualizer.sync_len(self.chat.len(), CHAT_INITIAL_MESSAGE_HEIGHT);
        self.chat_message_measured_heights.remove(&message_idx);
        self.chat_virtualizer.apply_measured_height(message_idx, estimated_height);
        self.sync_chat_virtualizer_compat_fields();
    }

    pub(crate) fn sync_chat_scroll_compat_fields(&mut self) {
        self.chat_scroll_offset_y = self.chat_virtualizer.relative_y();
        self.chat_scroll_viewport_h = self.chat_virtualizer.viewport_height_px();
    }

    /// 执行 protected_chat_ui_chunk_starts 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn protected_chat_ui_chunk_starts(&self) -> Vec<usize> {
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

        if let Some(chunk_start_idx) = self.active_session_view_state.pinned_chat_ui_chunk_start
            && chunk_start_idx < self.chat.len()
            && !chunk_starts.contains(&chunk_start_idx)
        {
            chunk_starts.push(chunk_start_idx);
        }

        if self.current_session_runtime().is_requesting
            && let Some(last_idx) = self.chat.len().checked_sub(1)
            && matches!(
                self.chat.get(last_idx).map(|message| message.role),
                Some(ChatRole::Assistant)
            )
        {
            push_protected_message_idx(&mut chunk_starts, self.chat.len(), last_idx);
        }

        for &message_idx in &self.chat_message_expanded {
            push_protected_message_idx(&mut chunk_starts, self.chat.len(), message_idx);
        }

        for &think_key in &self.chat_think_expanded {
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
    ) -> crate::app::components::chat_panel::chunk_loader::ChatChunkScheduleInput {
        let window = self.chat_virtualizer.window();
        let has_visible_window = window.visible_start_idx < window.visible_end_idx;
        let has_render_window = window.render_start_idx < window.render_end_idx;
        let visible_start_idx =
            if has_visible_window { window.visible_start_idx } else { start_idx };
        let visible_end_idx = if has_visible_window { window.visible_end_idx } else { end_idx };
        let render_start_idx = if has_render_window { window.render_start_idx } else { start_idx };
        let render_end_idx = if has_render_window { window.render_end_idx } else { end_idx };
        let anchor_idx = self
            .chat_virtualizer
            .anchor()
            .map(|anchor| anchor.index.min(self.chat.len().saturating_sub(1)));
        let direction_anchor_idx = anchor_idx.or_else(|| {
            chat_ui_visible_window_bounds(self.chat.len(), visible_start_idx, visible_end_idx)
                .map(|(start_idx, end_idx)| start_idx + (end_idx - start_idx - 1) / 2)
        });
        let scroll_direction = direction_anchor_idx
            .map(chat_ui_chunk_start_idx)
            .and_then(|anchor_chunk_start| {
                self.active_session_view_state
                    .last_visited_chat_ui_chunk_start
                    .map(|last_chunk_start| anchor_chunk_start.cmp(&last_chunk_start))
            })
            .map(|ordering| match ordering {
                Ordering::Greater => {
                    crate::app::components::chat_panel::chunk_loader::ChatChunkScrollDirection::Down
                }
                Ordering::Less => {
                    crate::app::components::chat_panel::chunk_loader::ChatChunkScrollDirection::Up
                }
                Ordering::Equal => crate::app::components::chat_panel::chunk_loader::
                    ChatChunkScrollDirection::Stationary,
            })
            .unwrap_or(
                crate::app::components::chat_panel::chunk_loader::ChatChunkScrollDirection::Stationary,
            );

        crate::app::components::chat_panel::chunk_loader::ChatChunkScheduleInput {
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

    /// 执行 pin_chat_ui_chunk 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn pin_chat_ui_chunk(&mut self, chunk_start_idx: Option<usize>) {
        self.active_session_view_state.pinned_chat_ui_chunk_start = chunk_start_idx
            .filter(|_| !self.chat.is_empty())
            .map(|idx| chat_ui_chunk_start_idx(idx.min(self.chat.len().saturating_sub(1))));
    }

    fn clear_chat_ui_chunk_range(
        &mut self,
        chunk_start_idx: usize,
        chunk_end_idx: usize,
        drop_interaction_state: bool,
    ) {
        for idx in chunk_start_idx..chunk_end_idx {
            self.chat_render_cache.remove(&idx);
            if idx < self.chat_visible_text_cache.len() {
                self.chat_visible_text_cache[idx] = None;
            }
            if idx < self.chat_copy_hash_cache.len() {
                self.chat_copy_hash_cache[idx] = None;
            }
            if idx < self.chat_message_editors.len() {
                self.chat_message_editors[idx] = text_editor::Content::new();
            }
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
            if self
                .tool_detail_dialog
                .as_ref()
                .is_some_and(|dialog| (chunk_start_idx..chunk_end_idx).contains(&dialog.msg_idx))
            {
                self.tool_detail_dialog = None;
            }
        }
    }

    /// 执行 unload_chat_ui_chunk 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn unload_chat_ui_chunk(&mut self, chunk_start_idx: usize) {
        let (chunk_start_idx, chunk_end_idx) =
            chat_ui_chunk_bounds(self.chat.len(), chunk_start_idx);
        if chunk_start_idx >= chunk_end_idx {
            return;
        }

        self.active_session_view_state.prepared_chat_ui_chunks.remove(&chunk_start_idx);
        self.active_session_view_state.preparing_chat_ui_chunks.remove(&chunk_start_idx);
        self.active_session_view_state.dirty_preparing_chat_ui_chunks.remove(&chunk_start_idx);
        self.clear_chat_ui_chunk_range(chunk_start_idx, chunk_end_idx, false);
        self.refine_chat_message_estimated_heights(chunk_start_idx, chunk_end_idx);
    }

    /// 执行 visible_chat_message_window 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn visible_chat_message_window(&self) -> (usize, usize) {
        let window = self.chat_virtualizer.window();
        (window.visible_start_idx, window.visible_end_idx)
    }

    /// 执行 prune_chat_heavy_editor_caches 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn prune_chat_heavy_editor_caches(&mut self, start_idx: usize, end_idx: usize) {
        let schedule_input = self.chat_chunk_schedule_input(start_idx, end_idx);
        let (keep_start, keep_end) =
            crate::app::components::chat_panel::chunk_loader::heavy_cache_keep_bounds_for_schedule(
                &schedule_input,
                CHAT_HEAVY_EDITOR_CACHE_MESSAGE_MARGIN,
            );
        let expanded_messages = self.chat_message_expanded.clone();
        let expanded_thinking = self.chat_think_expanded.clone();
        let keep_message = |message_idx: usize| {
            (keep_start..keep_end).contains(&message_idx)
                || expanded_messages.contains(&message_idx)
        };

        self.chat_special_text_editors.retain(|key, _| keep_message((*key >> 32) as usize));
        self.chat_markdown_blocks.retain(|key, _| keep_message((*key >> 32) as usize));
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

    /// 执行 prune_chat_ui_chunks 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn prune_chat_ui_chunks(&mut self, start_idx: usize, end_idx: usize) {
        let retained_chunk_starts =
            crate::app::components::chat_panel::chunk_loader::retained_chunk_starts_for_schedule(
                &self.chat_chunk_schedule_input(start_idx, end_idx),
            );
        let eviction_chunk_starts =
            crate::app::components::chat_panel::chunk_loader::eviction_chunk_starts(
                &self.active_session_view_state.prepared_chat_ui_chunks,
                &retained_chunk_starts,
            );

        for chunk_start_idx in eviction_chunk_starts {
            self.unload_chat_ui_chunk(chunk_start_idx);
        }
    }

    /// 执行 preferred_base_chat_ui_chunk_start 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn preferred_base_chat_ui_chunk_start(&self) -> usize {
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

    #[allow(dead_code)]
    /// 执行 collect_chat_ui_chunk_starts 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn collect_chat_ui_chunk_starts(
        &self,
        start_idx: usize,
        end_idx: usize,
        include_neighbors: bool,
    ) -> Vec<usize> {
        crate::app::components::chat_panel::chunk_loader::prioritized_chunk_starts(
            &self.chat_chunk_schedule_input(start_idx, end_idx),
            include_neighbors,
        )
    }

    /// 执行 pending_chat_ui_chunk_starts 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn pending_chat_ui_chunk_starts(
        &mut self,
        start_idx: usize,
        end_idx: usize,
        include_neighbors: bool,
    ) -> Vec<usize> {
        let schedule_input = self.chat_chunk_schedule_input(start_idx, end_idx);
        let chunk_starts =
            crate::app::components::chat_panel::chunk_loader::prioritized_chunk_starts(
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
            self.active_session_view_state.last_visited_chat_ui_chunk_start =
                Some(anchor_chunk_start);
        }
        chunk_starts
            .into_iter()
            .filter(|chunk_start_idx| {
                !self.active_session_view_state.prepared_chat_ui_chunks.contains(chunk_start_idx)
                    && !self
                        .active_session_view_state
                        .preparing_chat_ui_chunks
                        .contains(chunk_start_idx)
            })
            .collect()
    }

    /// 执行 mark_chat_ui_chunks_preparing 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn mark_chat_ui_chunks_preparing(&mut self, chunk_starts: &[usize]) {
        for &chunk_start_idx in chunk_starts {
            self.active_session_view_state.preparing_chat_ui_chunks.insert(chunk_start_idx);
            self.active_session_view_state.dirty_preparing_chat_ui_chunks.remove(&chunk_start_idx);
        }
    }

    /// 执行 invalidate_chat_ui_chunk 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn invalidate_chat_ui_chunk(&mut self, chunk_start_idx: usize) {
        let (chunk_start_idx, chunk_end_idx) =
            chat_ui_chunk_bounds(self.chat.len(), chunk_start_idx);
        if chunk_start_idx >= chunk_end_idx {
            return;
        }

        self.active_session_view_state.prepared_chat_ui_chunks.remove(&chunk_start_idx);
        self.active_session_view_state.preparing_chat_ui_chunks.remove(&chunk_start_idx);
        self.active_session_view_state.dirty_preparing_chat_ui_chunks.remove(&chunk_start_idx);
        self.clear_chat_ui_chunk_range(chunk_start_idx, chunk_end_idx, true);
        self.refine_chat_message_estimated_heights(chunk_start_idx, chunk_end_idx);
    }

    /// 执行 invalidate_chat_ui_for_message_idx 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn invalidate_chat_ui_for_message_idx(&mut self, idx: usize) {
        if idx < self.chat.len() {
            self.invalidate_chat_ui_chunk(chat_ui_chunk_start_idx(idx));
        }
    }

    /// 执行 sync_chat_message_estimated_heights_len 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn sync_chat_message_estimated_heights_len(&mut self) {
        self.sync_active_session_user_question_index();
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

    /// 执行 refine_chat_message_estimated_heights 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn refine_chat_message_estimated_heights(
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

    /// 执行 rebuild_chat_message_estimated_heights 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn rebuild_chat_message_estimated_heights(&mut self) {
        let heights = vec![CHAT_INITIAL_MESSAGE_HEIGHT; self.chat.len()];
        self.chat_virtualizer.set_estimated_heights(&heights);
        self.sync_chat_virtualizer_compat_fields();
    }

    /// 执行 invalidate_chat_ui_state 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub fn invalidate_chat_ui_state(&mut self) {
        self.chat_render_cache.clear();
        self.chat_visible_text_cache.clear();
        self.chat_copy_hash_cache.clear();
        self.active_session_view_state.message_meta_texts.clear();
        self.active_session_view_state.user_question_markers.clear();
        self.active_session_view_state.user_question_indexed_chat_len = 0;
        self.active_session_view_state.prepared_chat_ui_chunks.clear();
        self.active_session_view_state.preparing_chat_ui_chunks.clear();
        self.active_session_view_state.dirty_preparing_chat_ui_chunks.clear();
        self.active_session_view_state.last_visited_chat_ui_chunk_start = None;
        self.active_session_view_state.pinned_chat_ui_chunk_start = None;
        self.chat_message_expanded.clear();
        self.chat_height_index.clear();
        self.chat_virtualizer.set_estimated_heights(&[]);
        self.sync_chat_virtualizer_compat_fields();
        self.chat_message_measured_heights.clear();
        self.chat_message_editors.clear();
        self.chat_special_text_editors.clear();
        self.chat_markdown_blocks.clear();
        self.chat_markdown_block_editors.clear();
        self.chat_tool_text_editors.clear();
        self.chat_think_editors.clear();
        self.chat_think_expanded.clear();
        self.chat_think_collapsed.clear();
        self.chat_think_hovered_idx = None;
        self.chat_tool_file_expanded.clear();
        self.chat_tool_file_hovered = None;
        self.chat_tool_expanded.clear();
        self.chat_tool_hovered_idx = None;
        self.chat_explore_expanded.clear();
        self.chat_explore_summary_animations.clear();
        self.tool_detail_dialog = None;
        self.chat_think_scroll_ids.clear();
        self.chat_reset_menu_idx = None;
        self.chat_fork_dialog_idx = None;
        self.rebuild_chat_message_estimated_heights();
    }

    /// 执行 apply_prepared_chat_ui_phase 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
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

        self.active_session_view_state.preparing_chat_ui_chunks.remove(&chunk_start_idx);
        self.active_session_view_state.dirty_preparing_chat_ui_chunks.remove(&chunk_start_idx);
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
            self.chat_message_editors[idx] = text_editor::Content::new();
            let expected_hash = crate::app::components::chat_panel::message_view::hash_chat_content(
                &self.chat[idx].content,
            );
            let Some(render_entry) = render_cache.get(&idx) else {
                is_chunk_fresh = false;
                self.chat_render_cache.remove(&idx);
                self.chat_explore_summary_animations.retain(|key, _| (*key >> 64) as usize != idx);
                continue;
            };

            if render_entry.content_hash != expected_hash {
                is_chunk_fresh = false;
                self.chat_render_cache.remove(&idx);
                self.chat_explore_summary_animations.retain(|key, _| (*key >> 64) as usize != idx);
                continue;
            }

            self.chat_render_cache.insert(idx, render_entry.clone());
            sync_chat_aux_text_editors_for_message(self, idx, render_entry);
            let offset = idx - chunk_start_idx;
            self.chat_visible_text_cache[idx] = visible_texts.get(offset).cloned().flatten();
            self.chat_copy_hash_cache[idx] = copy_hashes.get(offset).copied().flatten();
            if let Some(Some(text)) = message_editor_texts.get(offset) {
                self.chat_message_editors[idx] = text_editor::Content::with_text(text);
            }
        }

        if is_chunk_fresh {
            self.active_session_view_state.prepared_chat_ui_chunks.insert(chunk_start_idx);
        } else {
            self.active_session_view_state.prepared_chat_ui_chunks.remove(&chunk_start_idx);
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

    fn apply_prepared_chat_ui_message(&mut self, prepared: PreparedChatUiMessage) {
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
            crate::app::components::chat_panel::message_view::hash_chat_content(&message.content);
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
        self.chat_markdown_blocks.retain(|key, _| (*key >> 32) as usize != message_idx);
        self.chat_markdown_block_editors.retain(|key, _| (*key >> 64) as usize != message_idx);
        self.chat_tool_text_editors.retain(|key, _| (*key >> 64) as usize != message_idx);
        self.chat_think_editors.retain(|key, _| (*key >> 32) as usize != message_idx);

        self.chat_render_cache.insert(message_idx, render_cache.clone());
        sync_chat_aux_text_editors_for_message(self, message_idx, &render_cache);
        self.chat_visible_text_cache[message_idx] = Some(visible_text);
        self.chat_copy_hash_cache[message_idx] = Some(copy_hash);
        self.chat_message_editors[message_idx] = message_editor_text
            .as_deref()
            .map(text_editor::Content::with_text)
            .unwrap_or_else(text_editor::Content::new);
        self.refine_chat_message_estimated_heights(message_idx, message_idx + 1);
    }

    /// 执行 pending_chat_message_editors_window_chunks 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn pending_chat_message_editors_window_chunks(
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

#[cfg(test)]
#[path = "chat_ui_tests.rs"]
mod chat_ui_tests;
