//! 聊天首屏派生、编辑器缓存与分块状态更新。
use session_ui_types::{ChatMessage, ChatRole};
use crate::render_cache::ChatRenderCacheEntry;
use crate::message_view::ParsedChatBlock;
use iced::widget::text_editor;
use std::collections::{HashMap, HashSet};
use std::cmp::Ordering;

pub struct ChatMarkdownSelection {
    pub session_id: String,
    pub message_idx: usize,
    pub text_idx: usize,
    pub text: String,
}

pub struct ChatUi<'a> {
    pub chat: &'a Vec<session_ui_types::ChatMessage>,
    pub chat_copy_hash_cache: &'a mut Vec<Option<u64>>,
    pub chat_explore_expanded: &'a mut HashSet<u64>,
    pub chat_explore_summary_animations: &'a mut HashMap<u128, desktop_chat_tools::tools::explore_input::ExploreSummaryAnimationState>,
    pub chat_height_index: &'a mut crate::height_index::ChatHeightIndex,
    pub chat_markdown_block_editors: &'a mut HashMap<u128, text_editor::Content>,
    pub chat_markdown_blocks: &'a mut HashMap<u64, Vec<crate::message_view::markdown::MarkdownBlock>>,
    pub chat_markdown_selection: &'a mut Option<ChatMarkdownSelection>,
    pub chat_message_editors: &'a mut Vec<text_editor::Content>,
    pub chat_message_estimated_heights: &'a mut Vec<f32>,
    pub chat_message_expanded: &'a mut HashSet<usize>,
    pub chat_message_measured_heights: &'a mut HashMap<usize, f32>,
    pub chat_render_cache: &'a mut HashMap<usize, ChatRenderCacheEntry>,
    pub chat_reset_menu_idx: &'a mut Option<usize>,
    pub chat_scroll_offset_y: &'a mut f32,
    pub chat_scroll_viewport_h: &'a mut f32,
    pub chat_special_text_editors: &'a mut HashMap<u64, text_editor::Content>,
    pub chat_think_collapsed: &'a mut HashSet<u64>,
    pub chat_think_editors: &'a mut HashMap<u64, text_editor::Content>,
    pub chat_think_expanded: &'a mut HashSet<u64>,
    pub chat_think_hovered_idx: &'a mut Option<u64>,
    pub chat_think_scroll_ids: &'a mut HashMap<u64, iced::widget::Id>,
    pub chat_tool_expanded: &'a mut HashSet<u64>,
    pub chat_tool_file_expanded: &'a mut HashSet<String>,
    pub chat_tool_file_hovered: &'a mut Option<String>,
    pub chat_tool_hovered_idx: &'a mut Option<u64>,
    pub chat_tool_text_editors: &'a mut HashMap<u128, text_editor::Content>,
    pub chat_virtualizer: &'a mut crate::virtualizer::ChatVirtualizer,
    pub chat_visible_text_cache: &'a mut Vec<Option<String>>,
    pub tool_detail_dialog: &'a mut Option<desktop_session::message::chat::input::tool_detail::ToolDetailDialog>,
    pub message_meta_texts: &'a mut Vec<Option<desktop_i18n::chat::MessageMeta>>,
    pub user_question_markers: &'a mut Vec<desktop_session::session::active::UserQuestionMarker>,
    pub user_question_indexed_chat_len: &'a mut usize,
    pub prepared_chat_ui_chunks: &'a mut HashSet<usize>,
    pub preparing_chat_ui_chunks: &'a mut HashSet<usize>,
    pub dirty_preparing_chat_ui_chunks: &'a mut HashSet<usize>,
    pub last_visited_chat_ui_chunk_start: &'a mut Option<usize>,
    pub pinned_chat_ui_chunk_start: &'a mut Option<usize>,
    pub desktop_locale: config_ui::DesktopLocale,
    pub dialogue_flow_show_reasoning_summary: bool,
    pub is_requesting: bool,
    pub timeline: Option<&'a desktop_session::session::chat_timeline::PagedChatTimeline>,
    pub window_width: f32,
    pub is_dark: bool,
}
pub struct ChatUiSchedule<'a> {
    pub chat: &'a Vec<session_ui_types::ChatMessage>,
    pub chat_virtualizer: &'a crate::virtualizer::ChatVirtualizer,
    pub pinned_chat_ui_chunk_start: &'a Option<usize>,
    pub last_visited_chat_ui_chunk_start: &'a Option<usize>,
    pub chat_message_expanded: &'a HashSet<usize>,
    pub chat_think_expanded: &'a HashSet<u64>,
    pub is_requesting: bool,
}
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
pub use crate::chunk_loader::CHAT_UI_CHUNK_SIZE;
const CHAT_HEAVY_EDITOR_CACHE_MESSAGE_MARGIN: usize = 24;
pub const CHAT_INITIAL_MESSAGE_HEIGHT: f32 = 120.0;

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
        let display_content = if message.role == ChatRole::User {
            session_text::strip_task_session_source_marker(&message.content)
        } else {
            std::borrow::Cow::Borrowed(message.content.as_str())
        };
        let visible = match message.role {
            ChatRole::Assistant => crate::message_view::think::split_think(display_content.as_ref()).1,
            ChatRole::Tool | ChatRole::User | ChatRole::System | ChatRole::Developer => {
                display_content.to_string()
            }
        };
        let copy_hash =
            crate::message_view::parse::hash_chat_content(&visible);
        let entry = crate::message_view::render_cache::build_render_cache_entry(
            display_content.as_ref(),
            &visible,
            copy_hash,
            show_reasoning_summary,
        );

        parsed.visible_texts[msg_idx] = Some(visible.clone());
        parsed.copy_hashes[msg_idx] = Some(copy_hash);
        let editor_text =
            crate::selectable_text::visible_editor_text_for_message(
                message.role,
                display_content.as_ref(),
                &visible,
                &entry.display_text,
            );
        if crate::selectable_text::read_only_editor_content(
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

pub fn cached_message_height(chat: &[ChatMessage], cache: &HashMap<usize, ChatRenderCacheEntry>, idx: usize) -> Option<f32> {
    let message = chat.get(idx)?;
    let entry = cache.get(&idx)?;
    (entry.content_hash == crate::message_view::parse::hash_chat_content(&message.content)).then_some(entry.estimated_expanded_height)
}
fn resolve_render_cache_height(app: &ChatUi<'_>, idx: usize) -> Option<f32> {
    cached_message_height(app.chat, app.chat_render_cache, idx)
}

fn estimate_chat_message_height(message: &ChatMessage) -> f32 {
    crate::message_view::text::estimate_message_height_rough(
        &message.content,
    )
}

fn revision_render_key(app: &ChatUi<'_>, idx: usize) -> Option<crate::render_cache::RevisionRenderKey> {
    let target = app.timeline?.target_at(idx)?;
    Some(crate::render_cache::RevisionRenderKey::new(target.key, target.revision,
        app.window_width, app.is_dark, app.dialogue_flow_show_reasoning_summary))
}

/// 执行 chat_ui_chunk_start_idx 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub use crate::chunk_loader::chat_ui_chunk_start_idx;

/// 执行 chat_ui_chunk_bounds 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub use crate::chunk_loader::chat_ui_chunk_bounds;

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
fn prioritize_chat_ui_chunk_starts(
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

/// 执行 explore_summary_animation_key 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn explore_summary_animation_key(msg_idx: usize, group_idx: usize) -> u128 {
    ((msg_idx as u128) << 64) | (group_idx as u128)
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

#[cfg(test)]
#[path="chat_ui_tests.rs"]
mod chat_ui_tests;

mod editors;
mod schedule;
mod cache;
mod update;
use editors::sync_chat_aux_text_editors_for_message;
