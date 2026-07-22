//! TUI v2 的派生状态入口。
//!
//! 当前 selectors 只依赖 `TuiState` 与内部模型，不依赖 renderer，
//! 这样后续无论是 Ratatui、snapshot 测试还是 shadow compare，都可以复用相同的
//! 最小计算逻辑。

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::mem;

use super::TuiState;
use crate::cli::tui_v2::model::{
    UiAssistantMessage, UiMessage, UiMessageId, UiOverlay, UiSearchMatch, UiStep, UiThinkingBlock,
    UiTokenUsage, UiToolCall, UiToolResult, UiTurnTerminal, UiUserMessage,
};
use unicode_width::UnicodeWidthChar;

/// 状态线可直接消费的汇总结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TuiStatusSummary {
    pub(crate) session_id: Option<String>,
    pub(crate) title: String,
    pub(crate) provider_name: Option<String>,
    pub(crate) model_name: Option<String>,
    pub(crate) message_count: usize,
    pub(crate) assistant_message_count: usize,
    pub(crate) step_count: usize,
    pub(crate) pending_questions: usize,
    pub(crate) todo_count: usize,
    pub(crate) overlay_depth: usize,
    pub(crate) prompt_busy: bool,
    pub(crate) turn_terminal: UiTurnTerminal,
    pub(crate) token_usage: UiTokenUsage,
}

/// renderer 可直接消费的 transcript 顶层项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TuiTranscriptItem<'a> {
    Standalone(&'a UiMessage),
    AssistantTurn(TuiAssistantTurnGroup<'a>),
}

/// 单条 assistant turn 的稳定派生视图。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TuiAssistantTurnGroup<'a> {
    pub(crate) assistant: &'a UiAssistantMessage,
    pub(crate) preface: Vec<TuiAssistantTurnEntry<'a>>,
    pub(crate) children: Vec<TuiAssistantTurnEntry<'a>>,
}

/// assistant turn 内部的子项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TuiAssistantTurnEntry<'a> {
    Thinking(&'a UiThinkingBlock),
    Step(&'a UiStep),
    Tool(TuiToolCallGroup<'a>),
    ToolResult(&'a UiToolResult),
    CollapsedTools(TuiCollapsedExploreResults<'a>),
}

/// tool call 与其结果的稳定归并视图。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TuiToolCallGroup<'a> {
    pub(crate) call: &'a UiToolCall,
    pub(crate) results: Vec<&'a UiToolResult>,
}

/// 当前最小折叠模型覆盖的 explore 工具类别。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TuiExploreToolKind {
    Read,
    Grep,
    Glob,
    SemanticSearch,
}

impl TuiExploreToolKind {
    fn label(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Grep => "grep",
            Self::Glob => "glob",
            Self::SemanticSearch => "semantic_search",
        }
    }
}

/// 折叠摘要里的工具计数。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TuiCollapsedToolCount {
    pub(crate) kind: TuiExploreToolKind,
    pub(crate) count: usize,
}

/// 连续 explore 类工具结果的折叠摘要。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TuiCollapsedExploreResults<'a> {
    pub(crate) summary: String,
    pub(crate) tool_counts: Vec<TuiCollapsedToolCount>,
    pub(crate) calls: Vec<TuiToolCallGroup<'a>>,
    pub(crate) total_results: usize,
}

/// renderer 与 footer 可共同消费的可见 transcript 窗口。
///
/// 当前 S4-2a 先稳定暴露窗口级元数据：
/// - 返回了哪些 transcript item
/// - 这些 item 落在完整 projection 的哪一段
/// - 它们覆盖了原始消息的哪一段
///
/// 这样 renderer 不需要再把“裸 item 列表 + 外部 scroll 状态”重新拼回窗口语义。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TuiVisibleTranscriptWindow<'a> {
    pub(crate) items: Vec<TuiTranscriptItem<'a>>,
    pub(crate) viewport_rows: u16,
    pub(crate) viewport_message_capacity: usize,
    pub(crate) top_message: usize,
    pub(crate) sticky_message: Option<usize>,
    pub(crate) sticky_prompt: Option<TuiStickyPromptSummary>,
    pub(crate) unseen_range: Option<TuiUnseenRangeSummary>,
    pub(crate) follow_tail: bool,
    pub(crate) total_items: usize,
    pub(crate) top_item_index: usize,
    pub(crate) start_item_index: usize,
    pub(crate) end_item_index: usize,
    pub(crate) covered_message_start: usize,
    pub(crate) covered_message_end: usize,
}

/// 当前 viewport 能力的稳定摘要。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TuiViewportSummary {
    pub(crate) rows: u16,
    pub(crate) message_capacity: usize,
}

impl TuiViewportSummary {
    pub(crate) fn label(self) -> String {
        format!("{}rows/{}messages", self.rows, self.message_capacity)
    }
}

/// 当前可见 transcript 窗口的稳定摘要。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TuiWindowSummary {
    pub(crate) top_message: usize,
    pub(crate) sticky_message: Option<usize>,
    pub(crate) follow_tail: bool,
    pub(crate) total_items: usize,
    pub(crate) top_item_index: usize,
    pub(crate) start_item_index: usize,
    pub(crate) end_item_index: usize,
    pub(crate) covered_message_start: usize,
    pub(crate) covered_message_end: usize,
}

impl TuiWindowSummary {
    pub(crate) fn coverage_label(self) -> String {
        if self.total_items == 0 || self.end_item_index == 0 {
            "-".to_string()
        } else {
            format!(
                "items {}..{}/{} msg {}..{}",
                self.start_item_index.saturating_add(1),
                self.end_item_index,
                self.total_items,
                self.covered_message_start,
                self.covered_message_end.saturating_sub(1)
            )
        }
    }

    pub(crate) fn sticky_label(self) -> String {
        match self.sticky_message {
            Some(anchor) => format!("sticky m{anchor}"),
            None if self.follow_tail => "tail follow".to_string(),
            None => "sticky -".to_string(),
        }
    }

    pub(crate) fn sticky_notice(self) -> Option<String> {
        self.sticky_message
            .map(|anchor| format!("message {anchor} is parked above the viewport host."))
    }

    pub(crate) fn has_sticky_anchor(self) -> bool {
        self.sticky_message.is_some()
    }

    pub(crate) fn follows_tail(self) -> bool {
        self.follow_tail
    }
}

/// 当前窗口顶部可复用的 sticky prompt 摘要。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TuiStickyPromptSummary {
    pub(crate) message_index: usize,
    pub(crate) preview: String,
}

impl TuiStickyPromptSummary {
    pub(crate) fn label(&self) -> String {
        format!("prompt m{}", self.message_index)
    }
}

/// 当前窗口的未读边界摘要。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TuiUnseenRangeSummary {
    pub(crate) first_unseen_message: usize,
    pub(crate) first_unseen_item_index: usize,
    pub(crate) unseen_message_count: usize,
    pub(crate) unseen_item_count: usize,
    pub(crate) boundary_in_window: bool,
}

impl TuiUnseenRangeSummary {
    pub(crate) fn pill_label(self) -> String {
        if self.unseen_message_count == 1 {
            "1 new message".to_string()
        } else {
            format!("{} new messages", self.unseen_message_count)
        }
    }

    pub(crate) fn divider_label(self) -> String {
        format!("{} below", self.pill_label())
    }
}

/// 搜索文本缓存中的单条消息索引。
#[derive(Debug, Clone, PartialEq, Eq)]
struct TuiSearchTextCacheEntry {
    text: String,
    ascii_lowercase_text: String,
}

/// 搜索弹层复用的可搜索文本缓存。
///
/// 当前只缓存“每条消息的可搜索文本”和其 ASCII lower 版本，
/// 让 search query 变更时不再重复做全文字符串拼接与小写化。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct TuiSearchTextCache {
    entries: Vec<TuiSearchTextCacheEntry>,
}

impl TuiSearchTextCache {
    fn entries(&self) -> &[TuiSearchTextCacheEntry] {
        &self.entries
    }

    pub(crate) fn is_aligned(&self, message_count: usize) -> bool {
        self.entries.len() == message_count
    }

    pub(crate) fn rebuild(&mut self, messages: &[UiMessage]) {
        self.entries = messages.iter().map(search_text_cache_entry_for_message).collect();
    }

    pub(crate) fn refresh_message(&mut self, messages: &[UiMessage], message_index: usize) {
        let Some(message) = messages.get(message_index) else {
            self.entries.truncate(messages.len());
            return;
        };

        let entry = search_text_cache_entry_for_message(message);
        match message_index.cmp(&self.entries.len()) {
            std::cmp::Ordering::Less => {
                self.entries[message_index] = entry;
            }
            std::cmp::Ordering::Equal => {
                self.entries.push(entry);
            }
            std::cmp::Ordering::Greater => {
                self.rebuild(messages);
            }
        }
        self.entries.truncate(messages.len());
    }
}

/// 单个宽度分组下的 transcript layout cache。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TuiTranscriptLayoutCacheBucket {
    pub(crate) content_width: u16,
    item_heights: Vec<usize>,
}

impl TuiTranscriptLayoutCacheBucket {
    fn item_heights(&self) -> &[usize] {
        &self.item_heights
    }
}

/// 按宽度分组的 transcript wrap/height cache。
///
/// 每个 bucket 都只缓存某个 content width 下的顶层 transcript item 行高，
/// 让 scroll/visible window 在相同宽度下复用估算结果，而不是每次重新 wrap 全段文本。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct TuiTranscriptLayoutCache {
    buckets: BTreeMap<u16, TuiTranscriptLayoutCacheBucket>,
}

impl TuiTranscriptLayoutCache {
    pub(crate) fn clear(&mut self) {
        self.buckets.clear();
    }

    pub(crate) fn bucket(&self, content_width: u16) -> Option<&TuiTranscriptLayoutCacheBucket> {
        self.buckets.get(&content_width)
    }

    pub(crate) fn rebuild_width(
        &mut self,
        messages: &[UiMessage],
        projection: &TuiTranscriptProjectionCache,
        content_width: u16,
    ) {
        if content_width == 0 {
            return;
        }

        self.buckets.insert(
            content_width,
            derive_transcript_layout_cache_bucket(messages, projection, content_width),
        );
    }

    pub(crate) fn refresh_message(
        &mut self,
        messages: &[UiMessage],
        projection: &TuiTranscriptProjectionCache,
        message_index: usize,
    ) {
        let Some(item_index) = projection.item_index_for_message(message_index) else {
            return;
        };

        for (content_width, bucket) in &mut self.buckets {
            if bucket.item_heights.len() != projection.len() {
                *bucket =
                    derive_transcript_layout_cache_bucket(messages, projection, *content_width);
                continue;
            }

            let Some(item) = projection.item(item_index) else {
                continue;
            };
            bucket.item_heights[item_index] =
                estimate_transcript_projection_item_height(messages, item, *content_width);
        }
    }
}

impl<'a> TuiVisibleTranscriptWindow<'a> {
    fn empty(state: &TuiState) -> Self {
        Self {
            items: Vec::new(),
            viewport_rows: state.scroll.viewport_height,
            viewport_message_capacity: 0,
            top_message: state.scroll.top_message,
            sticky_message: state.scroll.sticky_message,
            sticky_prompt: None,
            unseen_range: None,
            follow_tail: state.scroll.follow_tail,
            total_items: 0,
            top_item_index: 0,
            start_item_index: 0,
            end_item_index: 0,
            covered_message_start: 0,
            covered_message_end: 0,
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub(crate) fn len(&self) -> usize {
        self.items.len()
    }

    pub(crate) fn visible_items(&self) -> &[TuiTranscriptItem<'a>] {
        let top_offset = self.top_item_index.saturating_sub(self.start_item_index);
        &self.items[top_offset.min(self.items.len())..]
    }

    pub(crate) fn viewport_summary(&self) -> TuiViewportSummary {
        TuiViewportSummary {
            rows: self.viewport_rows,
            message_capacity: self.viewport_message_capacity,
        }
    }

    pub(crate) fn window_summary(&self) -> TuiWindowSummary {
        TuiWindowSummary {
            top_message: self.top_message,
            sticky_message: self.sticky_message,
            follow_tail: self.follow_tail,
            total_items: self.total_items,
            top_item_index: self.top_item_index,
            start_item_index: self.start_item_index,
            end_item_index: self.end_item_index,
            covered_message_start: self.covered_message_start,
            covered_message_end: self.covered_message_end,
        }
    }

    pub(crate) fn sticky_prompt(&self) -> Option<&TuiStickyPromptSummary> {
        self.sticky_prompt.as_ref()
    }

    pub(crate) fn unseen_range(&self) -> Option<TuiUnseenRangeSummary> {
        self.unseen_range
    }
}

/// grouped transcript 的稳定投影视图缓存。
///
/// 当前 S4-2a 只先缓存“顶层项边界”和“消息锚点”，
/// 让滚动、visible window 与 renderer 共享同一份投影结果，
/// 避免每次交互都从原始消息序列重新分组整段 transcript。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct TuiTranscriptProjectionCache {
    items: Vec<TuiTranscriptProjectionItem>,
    anchors: Vec<usize>,
}

impl TuiTranscriptProjectionCache {
    fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    fn len(&self) -> usize {
        self.items.len()
    }

    fn items(&self) -> &[TuiTranscriptProjectionItem] {
        &self.items
    }

    pub(crate) fn anchors(&self) -> &[usize] {
        &self.anchors
    }

    pub(crate) fn item(&self, item_index: usize) -> Option<&TuiTranscriptProjectionItem> {
        self.items.get(item_index)
    }

    pub(crate) fn item_index_for_message(&self, message_index: usize) -> Option<usize> {
        self.items.iter().position(|item| item.span().contains(message_index))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct IndexedUiMessage<'a> {
    message: &'a UiMessage,
    index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MessageWindowSpan {
    start: usize,
    end: usize,
}

impl MessageWindowSpan {
    fn single(index: usize) -> Self {
        Self { start: index, end: index.saturating_add(1) }
    }

    fn contains(self, index: usize) -> bool {
        self.start <= index && index < self.end
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TuiTranscriptProjectionItem {
    Standalone {
        message_index: usize,
        span: MessageWindowSpan,
    },
    AssistantTurn {
        assistant_index: usize,
        span: MessageWindowSpan,
        preface_message_indices: Vec<usize>,
        child_message_indices: Vec<usize>,
    },
}

impl TuiTranscriptProjectionItem {
    fn span(&self) -> MessageWindowSpan {
        match self {
            Self::Standalone { span, .. } | Self::AssistantTurn { span, .. } => *span,
        }
    }

    fn materialize<'a>(&self, messages: &'a [UiMessage]) -> TuiTranscriptItem<'a> {
        match self {
            Self::Standalone { message_index, .. } => TuiTranscriptItem::Standalone(
                messages
                    .get(*message_index)
                    .expect("transcript standalone index must stay in sync with messages"),
            ),
            Self::AssistantTurn {
                assistant_index,
                preface_message_indices,
                child_message_indices,
                ..
            } => {
                let assistant = messages
                    .get(*assistant_index)
                    .and_then(|message| match message {
                        UiMessage::Assistant(assistant) => Some(assistant),
                        _ => None,
                    })
                    .expect("transcript assistant index must point to assistant message");

                TuiTranscriptItem::AssistantTurn(TuiAssistantTurnGroup {
                    assistant,
                    preface: derive_turn_entries(message_indices_to_indexed_messages(
                        messages,
                        preface_message_indices,
                    )),
                    children: derive_turn_entries(message_indices_to_indexed_messages(
                        messages,
                        child_message_indices,
                    )),
                })
            }
        }
    }
}

#[derive(Debug)]
enum TranscriptBuilderItem<'a> {
    Standalone(IndexedUiMessage<'a>),
    AssistantTurn(AssistantTurnBuilder<'a>),
}

#[derive(Debug)]
struct AssistantTurnBuilder<'a> {
    assistant_index: usize,
    preface_messages: Vec<IndexedUiMessage<'a>>,
    child_messages: Vec<IndexedUiMessage<'a>>,
}

/// 返回当前消息窗口的可见切片。
pub(crate) fn select_visible_message_window(state: &TuiState) -> &[UiMessage] {
    if state.messages.is_empty() {
        return &state.messages[0..0];
    }

    if state.scroll.viewport_messages == 0 {
        return &state.messages[0..0];
    }

    let top = state.scroll.top_message.min(state.messages.len().saturating_sub(1));
    let start = top.saturating_sub(state.scroll.overscan);
    let end = top
        .saturating_add(state.scroll.viewport_messages)
        .saturating_add(state.scroll.overscan)
        .min(state.messages.len());
    &state.messages[start..end]
}

/// 返回当前 grouped transcript 窗口的可见切片。
///
/// 该窗口仍以 `scroll.top_message` 作为原始消息锚点，
/// 但会向上提升为“包含该消息的完整 transcript item”，避免 assistant turn
/// 在 thinking/tool/step 边界处被切开。
pub(crate) fn select_visible_grouped_transcript_window(
    state: &TuiState,
) -> TuiVisibleTranscriptWindow<'_> {
    let projection = &state.transcript;
    if projection.is_empty() {
        return TuiVisibleTranscriptWindow::empty(state);
    }

    let total_items = projection.len();
    let top_message = state.scroll.top_message.min(state.messages.len().saturating_sub(1));
    let top_item_index = transcript_item_index_for_message(projection.items(), top_message);
    let sticky_prompt = derive_sticky_prompt_summary(&state.messages, projection, top_item_index);

    if state.scroll.viewport_height == 0 {
        let covered_message_start =
            projection.items().first().map(|item| item.span().start).unwrap_or_default();
        let covered_message_end =
            projection.items().last().map(|item| item.span().end).unwrap_or_default();
        let unseen_range = derive_unseen_range_summary(
            &state.messages,
            projection,
            top_item_index,
            total_items,
            state.scroll.last_seen_message,
            state.scroll.follow_tail,
        );
        return TuiVisibleTranscriptWindow {
            items: materialize_transcript_items(&state.messages, projection.items()),
            viewport_rows: state.scroll.viewport_height,
            viewport_message_capacity: total_items.saturating_sub(top_item_index),
            top_message,
            sticky_message: state.scroll.sticky_message,
            sticky_prompt,
            unseen_range,
            follow_tail: state.scroll.follow_tail,
            total_items,
            top_item_index,
            start_item_index: 0,
            end_item_index: total_items,
            covered_message_start,
            covered_message_end,
        };
    }

    let start_item_index = top_item_index.saturating_sub(state.scroll.overscan);
    let visible_end_index = transcript_window_end_index_for_viewport(
        &state.messages,
        projection,
        &state.transcript_layout,
        top_item_index,
        state.scroll.viewport_width,
        state.scroll.viewport_height as usize,
    );
    let end_item_index =
        visible_end_index.saturating_add(state.scroll.overscan).min(projection.len());
    let items = projection.items()[start_item_index..end_item_index]
        .iter()
        .map(|item| item.materialize(&state.messages))
        .collect::<Vec<_>>();
    let covered_message_start = projection.items()[start_item_index].span().start;
    let covered_message_end = projection.items()[end_item_index.saturating_sub(1)].span().end;
    let unseen_range = derive_unseen_range_summary(
        &state.messages,
        projection,
        top_item_index,
        end_item_index,
        state.scroll.last_seen_message,
        state.scroll.follow_tail,
    );

    TuiVisibleTranscriptWindow {
        items,
        viewport_rows: state.scroll.viewport_height,
        viewport_message_capacity: visible_end_index.saturating_sub(top_item_index),
        top_message,
        sticky_message: state.scroll.sticky_message,
        sticky_prompt,
        unseen_range,
        follow_tail: state.scroll.follow_tail,
        total_items,
        top_item_index,
        start_item_index,
        end_item_index,
        covered_message_start,
        covered_message_end,
    }
}

/// 返回 grouped transcript 每个顶层项的原始消息锚点。
pub(crate) fn select_transcript_message_anchors(state: &TuiState) -> Vec<usize> {
    state.transcript.anchors().to_vec()
}

/// 基于扁平 `UiMessage` 序列派生 grouped/collapsed transcript。
pub(crate) fn select_grouped_transcript(state: &TuiState) -> Vec<TuiTranscriptItem<'_>> {
    materialize_transcript_items(&state.messages, state.transcript.items())
}

/// 按现有 `UiMessage` 元信息派生 grouped transcript。
pub(crate) fn derive_grouped_transcript(messages: &[UiMessage]) -> Vec<TuiTranscriptItem<'_>> {
    let projection = derive_transcript_projection_cache(messages);
    materialize_transcript_items(messages, projection.items())
}

/// 基于扁平 `UiMessage` 序列派生 grouped transcript 的消息锚点。
pub(crate) fn derive_transcript_message_anchors(messages: &[UiMessage]) -> Vec<usize> {
    derive_transcript_projection_cache(messages).anchors
}

pub(crate) fn derive_transcript_projection_cache(
    messages: &[UiMessage],
) -> TuiTranscriptProjectionCache {
    let mut items = Vec::new();
    let mut pending_preface = Vec::new();
    let mut pending_user_id: Option<&UiMessageId> = None;

    for (index, message) in messages.iter().enumerate() {
        let indexed_message = IndexedUiMessage { message, index };
        match message {
            UiMessage::User(user) => {
                flush_pending_preface(&mut items, &mut pending_preface);
                items.push(TranscriptBuilderItem::Standalone(indexed_message));
                pending_user_id = Some(&user.base.id);
            }
            UiMessage::Assistant(_) => {
                items.push(TranscriptBuilderItem::AssistantTurn(AssistantTurnBuilder {
                    assistant_index: index,
                    preface_messages: mem::take(&mut pending_preface),
                    child_messages: Vec::new(),
                }));
                pending_user_id = None;
            }
            UiMessage::Thinking(_)
            | UiMessage::Step(_)
            | UiMessage::ToolCall(_)
            | UiMessage::ToolResult(_) => {
                if let Some(TranscriptBuilderItem::AssistantTurn(turn)) = items.last_mut()
                    && assistant_turn_accepts_message(turn, message)
                {
                    turn.child_messages.push(indexed_message);
                    continue;
                }

                if can_stage_preface_message(message, pending_user_id, &pending_preface) {
                    pending_preface.push(indexed_message);
                    continue;
                }

                flush_pending_preface(&mut items, &mut pending_preface);
                items.push(TranscriptBuilderItem::Standalone(indexed_message));
                pending_user_id = None;
            }
            UiMessage::System(_) | UiMessage::Error(_) => {
                flush_pending_preface(&mut items, &mut pending_preface);
                items.push(TranscriptBuilderItem::Standalone(indexed_message));
                pending_user_id = None;
            }
        }
    }

    flush_pending_preface(&mut items, &mut pending_preface);

    let items = items.into_iter().map(finalize_transcript_projection_item).collect::<Vec<_>>();
    let anchors = items.iter().map(|item| item.span().start).collect();
    TuiTranscriptProjectionCache { items, anchors }
}

/// 汇总当前状态线关心的统计信息。
pub(crate) fn select_status_summary(state: &TuiState) -> TuiStatusSummary {
    let mut assistant_message_count = 0usize;
    let mut step_count = 0usize;
    let mut step_usage = UiTokenUsage::default();
    let mut assistant_usage = UiTokenUsage::default();

    for message in &state.messages {
        match message {
            UiMessage::Assistant(assistant) => {
                assistant_message_count = assistant_message_count.saturating_add(1);
                add_usage(&mut assistant_usage, &assistant.usage);
            }
            UiMessage::Step(step) => {
                step_count = step_count.saturating_add(1);
                add_usage(&mut step_usage, &step.usage);
            }
            _ => {}
        }
    }

    TuiStatusSummary {
        session_id: state.session.session_id.clone(),
        title: state.status.session_title.clone(),
        provider_name: state.status.provider_name.clone(),
        model_name: state.status.model_name.clone(),
        message_count: state.messages.len(),
        assistant_message_count,
        step_count,
        pending_questions: state.tasks.pending_questions.len(),
        todo_count: state
            .tasks
            .todo_overlay
            .as_ref()
            .map(|overlay| overlay.items.len())
            .unwrap_or_default(),
        overlay_depth: state.overlays.stack.len(),
        prompt_busy: state.prompt.is_busy(),
        turn_terminal: state.status.turn_terminal.clone(),
        token_usage: if step_count > 0 { step_usage } else { assistant_usage },
    }
}

/// 基于当前激活的搜索弹层派生搜索结果。
pub(crate) fn select_search_matches(state: &TuiState) -> Vec<UiSearchMatch> {
    let Some(UiOverlay::Search(search)) = state.overlays.active() else {
        return Vec::new();
    };
    if state.search_index.is_aligned(state.messages.len()) {
        derive_search_matches_from_cache(
            &state.messages,
            state.search_index.entries(),
            &search.query,
            search.case_sensitive,
        )
    } else {
        derive_search_matches(&state.messages, &search.query, search.case_sensitive)
    }
}

/// 按消息内容派生搜索命中列表。
pub(crate) fn derive_search_matches(
    messages: &[UiMessage],
    query: &str,
    case_sensitive: bool,
) -> Vec<UiSearchMatch> {
    let cache = derive_search_text_cache(messages);
    derive_search_matches_with_cache(messages, &cache, query, case_sensitive)
}

pub(crate) fn derive_search_text_cache(messages: &[UiMessage]) -> TuiSearchTextCache {
    let mut cache = TuiSearchTextCache::default();
    cache.rebuild(messages);
    cache
}

pub(crate) fn derive_search_matches_with_cache(
    messages: &[UiMessage],
    cache: &TuiSearchTextCache,
    query: &str,
    case_sensitive: bool,
) -> Vec<UiSearchMatch> {
    if !cache.is_aligned(messages.len()) {
        let cache = derive_search_text_cache(messages);
        return derive_search_matches_from_cache(messages, cache.entries(), query, case_sensitive);
    }

    derive_search_matches_from_cache(messages, cache.entries(), query, case_sensitive)
}

fn derive_search_matches_from_cache(
    messages: &[UiMessage],
    cache_entries: &[TuiSearchTextCacheEntry],
    query: &str,
    case_sensitive: bool,
) -> Vec<UiSearchMatch> {
    let normalized_query = query.trim();
    if normalized_query.is_empty() {
        return Vec::new();
    }

    let needle = if case_sensitive {
        normalized_query.to_string()
    } else {
        normalized_query.to_ascii_lowercase()
    };

    let mut matches = Vec::new();
    for (message, cache_entry) in messages.iter().zip(cache_entries.iter()) {
        let searchable = cache_entry.text.as_str();
        let haystack = if case_sensitive {
            cache_entry.text.as_str()
        } else {
            cache_entry.ascii_lowercase_text.as_str()
        };

        let mut search_start = 0usize;
        while let Some(relative_start) = haystack[search_start..].find(needle.as_str()) {
            let start = search_start.saturating_add(relative_start);
            let end = start.saturating_add(needle.len());
            matches.push(UiSearchMatch {
                message_id: Some(message.id().clone()),
                start,
                end,
                preview: excerpt_around(searchable, start, end),
            });
            search_start = end;
        }
    }

    matches
}

fn flush_pending_preface<'a>(
    items: &mut Vec<TranscriptBuilderItem<'a>>,
    pending_preface: &mut Vec<IndexedUiMessage<'a>>,
) {
    for message in pending_preface.drain(..) {
        items.push(TranscriptBuilderItem::Standalone(message));
    }
}

fn finalize_transcript_projection_item(
    item: TranscriptBuilderItem<'_>,
) -> TuiTranscriptProjectionItem {
    match item {
        TranscriptBuilderItem::Standalone(message) => TuiTranscriptProjectionItem::Standalone {
            message_index: message.index,
            span: MessageWindowSpan::single(message.index),
        },
        TranscriptBuilderItem::AssistantTurn(turn) => {
            let mut span = MessageWindowSpan::single(turn.assistant_index);
            if let Some(message) = turn.preface_messages.first() {
                span.start = message.index;
            }
            if let Some(message) = turn.child_messages.last() {
                span.end = message.index.saturating_add(1);
            }

            TuiTranscriptProjectionItem::AssistantTurn {
                assistant_index: turn.assistant_index,
                span,
                preface_message_indices: turn
                    .preface_messages
                    .into_iter()
                    .map(|message| message.index)
                    .collect(),
                child_message_indices: turn
                    .child_messages
                    .into_iter()
                    .map(|message| message.index)
                    .collect(),
            }
        }
    }
}

fn derive_sticky_prompt_summary(
    messages: &[UiMessage],
    projection: &TuiTranscriptProjectionCache,
    top_item_index: usize,
) -> Option<TuiStickyPromptSummary> {
    let assistant_index = match projection.item(top_item_index)? {
        TuiTranscriptProjectionItem::AssistantTurn { assistant_index, .. } => *assistant_index,
        TuiTranscriptProjectionItem::Standalone { .. } => return None,
    };
    let prompt_message_index = user_prompt_index_for_assistant(messages, assistant_index)?;
    let prompt_item_index = projection.item_index_for_message(prompt_message_index)?;
    if prompt_item_index >= top_item_index {
        return None;
    }

    let UiMessage::User(prompt) = messages.get(prompt_message_index)? else {
        return None;
    };

    Some(TuiStickyPromptSummary {
        message_index: prompt_message_index,
        preview: prompt_preview(prompt),
    })
}

fn derive_unseen_range_summary(
    messages: &[UiMessage],
    projection: &TuiTranscriptProjectionCache,
    top_item_index: usize,
    end_item_index: usize,
    last_seen_message: Option<usize>,
    follow_tail: bool,
) -> Option<TuiUnseenRangeSummary> {
    if follow_tail {
        return None;
    }

    let last_seen_message = last_seen_message?;
    if messages.is_empty() || last_seen_message >= messages.len().saturating_sub(1) {
        return None;
    }

    let first_unseen_message = last_seen_message.saturating_add(1);
    let first_unseen_item_index =
        projection.items().iter().position(|item| item.span().end > first_unseen_message)?;

    Some(TuiUnseenRangeSummary {
        first_unseen_message,
        first_unseen_item_index,
        unseen_message_count: messages.len().saturating_sub(first_unseen_message),
        unseen_item_count: projection.len().saturating_sub(first_unseen_item_index),
        boundary_in_window: first_unseen_item_index >= top_item_index
            && first_unseen_item_index < end_item_index,
    })
}

fn user_prompt_index_for_assistant(
    messages: &[UiMessage],
    assistant_index: usize,
) -> Option<usize> {
    let UiMessage::Assistant(assistant) = messages.get(assistant_index)? else {
        return None;
    };

    if let Some(parent_id) = assistant.base.parent_id.as_ref()
        && let Some((index, _)) = messages.iter().enumerate().rev().find(
            |(_, message)| matches!(message, UiMessage::User(user) if &user.base.id == parent_id),
        )
    {
        return Some(index);
    }

    messages[..assistant_index]
        .iter()
        .enumerate()
        .rev()
        .find_map(|(index, message)| matches!(message, UiMessage::User(_)).then_some(index))
}

fn prompt_preview(prompt: &UiUserMessage) -> String {
    let head = prompt
        .text
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("(empty prompt)");
    truncate_preview(head, 72)
}

fn truncate_preview(value: &str, max_chars: usize) -> String {
    let char_count = value.chars().count();
    if char_count <= max_chars {
        return value.to_string();
    }

    if max_chars <= 3 {
        return value.chars().take(max_chars).collect();
    }

    let prefix = value.chars().take(max_chars.saturating_sub(3)).collect::<String>();
    format!("{prefix}...")
}

fn materialize_transcript_items<'a>(
    messages: &'a [UiMessage],
    items: &[TuiTranscriptProjectionItem],
) -> Vec<TuiTranscriptItem<'a>> {
    items.iter().map(|item| item.materialize(messages)).collect()
}

fn message_indices_to_indexed_messages<'a>(
    messages: &'a [UiMessage],
    indices: &[usize],
) -> Vec<IndexedUiMessage<'a>> {
    indices
        .iter()
        .map(|index| IndexedUiMessage {
            message: messages
                .get(*index)
                .expect("transcript child index must stay in sync with messages"),
            index: *index,
        })
        .collect()
}

fn derive_turn_entries(messages: Vec<IndexedUiMessage<'_>>) -> Vec<TuiAssistantTurnEntry<'_>> {
    let mut entries = Vec::new();

    for message in messages {
        match message.message {
            UiMessage::Thinking(thinking) => {
                entries.push(TuiAssistantTurnEntry::Thinking(thinking));
            }
            UiMessage::Step(step) => entries.push(TuiAssistantTurnEntry::Step(step)),
            UiMessage::ToolCall(call) => entries
                .push(TuiAssistantTurnEntry::Tool(TuiToolCallGroup { call, results: Vec::new() })),
            UiMessage::ToolResult(result) => {
                if let Some(TuiAssistantTurnEntry::Tool(tool_group)) = entries.last_mut()
                    && tool_result_matches_call(tool_group.call, result)
                {
                    tool_group.results.push(result);
                    continue;
                }
                entries.push(TuiAssistantTurnEntry::ToolResult(result));
            }
            _ => {}
        }
    }

    collapse_explore_entries(entries)
}

fn collapse_explore_entries(
    entries: Vec<TuiAssistantTurnEntry<'_>>,
) -> Vec<TuiAssistantTurnEntry<'_>> {
    let mut collapsed_entries = Vec::new();
    let mut pending_run = Vec::new();

    for entry in entries {
        match entry {
            TuiAssistantTurnEntry::Tool(tool_group)
                if collapse_kind_for_tool_group(&tool_group).is_some() =>
            {
                pending_run.push(tool_group);
            }
            other => {
                flush_collapsible_tool_run(&mut collapsed_entries, &mut pending_run);
                collapsed_entries.push(other);
            }
        }
    }

    flush_collapsible_tool_run(&mut collapsed_entries, &mut pending_run);
    collapsed_entries
}

fn flush_collapsible_tool_run<'a>(
    collapsed_entries: &mut Vec<TuiAssistantTurnEntry<'a>>,
    pending_run: &mut Vec<TuiToolCallGroup<'a>>,
) {
    match pending_run.len() {
        0 => {}
        1 => {
            let tool_group = pending_run.pop().expect("single pending tool group must exist");
            collapsed_entries.push(TuiAssistantTurnEntry::Tool(tool_group));
        }
        _ => collapsed_entries.push(TuiAssistantTurnEntry::CollapsedTools(
            build_collapsed_explore_results(mem::take(pending_run)),
        )),
    }
}

fn build_collapsed_explore_results(
    calls: Vec<TuiToolCallGroup<'_>>,
) -> TuiCollapsedExploreResults<'_> {
    let mut tool_counts: Vec<TuiCollapsedToolCount> = Vec::new();
    let mut total_results = 0usize;

    for call in &calls {
        let Some(kind) = collapse_kind_for_tool_group(call) else {
            continue;
        };

        total_results = total_results.saturating_add(call.results.len());
        if let Some(existing) = tool_counts.iter_mut().find(|count| count.kind == kind) {
            existing.count = existing.count.saturating_add(1);
        } else {
            tool_counts.push(TuiCollapsedToolCount { kind, count: 1 });
        }
    }

    let summary = tool_counts
        .iter()
        .map(|count| format!("{} x{}", count.kind.label(), count.count))
        .collect::<Vec<_>>()
        .join(", ");

    TuiCollapsedExploreResults {
        summary: format!("explore results: {summary}"),
        tool_counts,
        calls,
        total_results,
    }
}

fn assistant_turn_accepts_message(turn: &AssistantTurnBuilder<'_>, message: &UiMessage) -> bool {
    match message {
        UiMessage::Thinking(_) | UiMessage::Step(_) | UiMessage::ToolCall(_) => true,
        UiMessage::ToolResult(result) => {
            tool_result_matches_messages(result, &turn.child_messages)
                || tool_result_matches_messages(result, &turn.preface_messages)
        }
        _ => false,
    }
}

fn can_stage_preface_message(
    message: &UiMessage,
    pending_user_id: Option<&UiMessageId>,
    pending_preface: &[IndexedUiMessage<'_>],
) -> bool {
    let Some(user_id) = pending_user_id else {
        return false;
    };

    match message {
        UiMessage::Thinking(_) | UiMessage::ToolCall(_) => {
            message.base().parent_id.as_ref().is_some_and(|parent_id| parent_id == user_id)
        }
        UiMessage::ToolResult(result) => tool_result_matches_messages(result, pending_preface),
        _ => false,
    }
}

fn tool_result_matches_messages(result: &UiToolResult, messages: &[IndexedUiMessage<'_>]) -> bool {
    messages.iter().rev().any(|message| match message {
        IndexedUiMessage { message: UiMessage::ToolCall(call), .. } => {
            tool_result_matches_call(call, result)
        }
        _ => false,
    })
}

fn transcript_item_index_for_message(
    items: &[TuiTranscriptProjectionItem],
    message_index: usize,
) -> usize {
    items
        .iter()
        .position(|item| item.span().contains(message_index))
        .unwrap_or_else(|| items.len().saturating_sub(1))
}

fn transcript_window_end_index(
    items: &[TuiTranscriptProjectionItem],
    top_item_index: usize,
    top_message_index: usize,
    viewport_messages: usize,
) -> usize {
    let target_end = top_message_index.saturating_add(viewport_messages.max(1));
    let mut covered_until = top_message_index;
    let mut end_index = top_item_index;

    while end_index < items.len() {
        covered_until = covered_until.max(items[end_index].span().end);
        end_index = end_index.saturating_add(1);
        if covered_until >= target_end {
            break;
        }
    }

    end_index
}

fn transcript_window_end_index_by_rows(
    item_heights: &[usize],
    top_item_index: usize,
    viewport_rows: usize,
) -> usize {
    let target_rows = viewport_rows.max(1);
    let mut covered_rows = 0usize;
    let mut end_index = top_item_index;

    while end_index < item_heights.len() {
        covered_rows = covered_rows.saturating_add(item_heights[end_index].max(1));
        end_index = end_index.saturating_add(1);
        if covered_rows >= target_rows {
            break;
        }
    }

    end_index
}

fn transcript_window_end_index_for_viewport(
    messages: &[UiMessage],
    projection: &TuiTranscriptProjectionCache,
    layout_cache: &TuiTranscriptLayoutCache,
    top_item_index: usize,
    content_width: u16,
    viewport_rows: usize,
) -> usize {
    if let Some(bucket) = layout_cache
        .bucket(content_width)
        .filter(|bucket| bucket.item_heights().len() == projection.len())
    {
        return transcript_window_end_index_by_rows(
            bucket.item_heights(),
            top_item_index,
            viewport_rows,
        );
    }

    transcript_window_end_index_by_estimated_rows(
        messages,
        projection.items(),
        top_item_index,
        content_width,
        viewport_rows,
    )
}

fn transcript_window_end_index_by_estimated_rows(
    messages: &[UiMessage],
    items: &[TuiTranscriptProjectionItem],
    top_item_index: usize,
    content_width: u16,
    viewport_rows: usize,
) -> usize {
    let target_rows = viewport_rows.max(1);
    let mut covered_rows = 0usize;
    let mut end_index = top_item_index;

    while end_index < items.len() {
        covered_rows = covered_rows.saturating_add(estimate_transcript_projection_item_height(
            messages,
            &items[end_index],
            content_width,
        ));
        end_index = end_index.saturating_add(1);
        if covered_rows >= target_rows {
            break;
        }
    }

    end_index
}

fn tool_result_matches_call(call: &UiToolCall, result: &UiToolResult) -> bool {
    if result.base.parent_id.as_ref().is_some_and(|parent_id| parent_id == &call.base.id) {
        return true;
    }

    match (call.call_id.as_deref(), result.call_id.as_deref()) {
        (Some(call_id), Some(result_call_id)) if call_id == result_call_id => true,
        _ => result.base.parent_id.is_none() && call.tool_name == result.tool_name,
    }
}

fn collapse_kind_for_tool_group(tool_group: &TuiToolCallGroup<'_>) -> Option<TuiExploreToolKind> {
    if tool_group.results.is_empty() || tool_group.results.iter().any(|result| result.is_error) {
        return None;
    }

    collapse_kind_for_tool_name(tool_group.call.tool_name.as_str())
}

fn collapse_kind_for_tool_name(tool_name: &str) -> Option<TuiExploreToolKind> {
    match tool_name.trim().to_ascii_lowercase().as_str() {
        "read" | "read_file" => Some(TuiExploreToolKind::Read),
        "grep" | "grep_search" => Some(TuiExploreToolKind::Grep),
        "glob" | "file_search" => Some(TuiExploreToolKind::Glob),
        "semantic_search" => Some(TuiExploreToolKind::SemanticSearch),
        _ => None,
    }
}

fn searchable_text(message: &UiMessage) -> Cow<'_, str> {
    match message {
        UiMessage::User(message) => Cow::Borrowed(message.text.as_str()),
        UiMessage::Assistant(message) => Cow::Borrowed(message.text.as_str()),
        UiMessage::ToolCall(message) => message
            .summary
            .as_deref()
            .map(Cow::Borrowed)
            .or_else(|| message.arguments.as_deref().map(Cow::Borrowed))
            .unwrap_or_else(|| Cow::Borrowed(message.tool_name.as_str())),
        UiMessage::ToolResult(message) => Cow::Borrowed(message.content.as_str()),
        UiMessage::Thinking(message) => {
            if let Some(summary) = message.summary.as_deref() {
                Cow::Borrowed(summary)
            } else {
                Cow::Borrowed(message.content.as_str())
            }
        }
        UiMessage::Step(step) => Cow::Owned(format!(
            "step {} {} {}",
            step.step_index,
            step.model.as_deref().unwrap_or(""),
            step.finish_reason.as_deref().unwrap_or("")
        )),
        UiMessage::System(message) => Cow::Borrowed(message.text.as_str()),
        UiMessage::Error(message) => Cow::Borrowed(message.message.as_str()),
    }
}

fn search_text_cache_entry_for_message(message: &UiMessage) -> TuiSearchTextCacheEntry {
    let text = searchable_text(message).into_owned();
    let ascii_lowercase_text = text.to_ascii_lowercase();
    TuiSearchTextCacheEntry { text, ascii_lowercase_text }
}

fn derive_transcript_layout_cache_bucket(
    messages: &[UiMessage],
    projection: &TuiTranscriptProjectionCache,
    content_width: u16,
) -> TuiTranscriptLayoutCacheBucket {
    let item_heights = projection
        .items()
        .iter()
        .map(|item| estimate_transcript_projection_item_height(messages, item, content_width))
        .collect();

    TuiTranscriptLayoutCacheBucket { content_width, item_heights }
}

fn estimate_transcript_projection_item_height(
    messages: &[UiMessage],
    item: &TuiTranscriptProjectionItem,
    content_width: u16,
) -> usize {
    let item = item.materialize(messages);
    estimate_transcript_item_height(&item, content_width)
}

fn estimate_transcript_item_height(item: &TuiTranscriptItem<'_>, content_width: u16) -> usize {
    transcript_item_line_texts(item)
        .into_iter()
        .map(|line| wrapped_line_count(line.as_str(), content_width))
        .sum::<usize>()
        .saturating_add(1)
}

fn transcript_item_line_texts(item: &TuiTranscriptItem<'_>) -> Vec<String> {
    match item {
        TuiTranscriptItem::Standalone(message) => vec![message_line_text(message)],
        TuiTranscriptItem::AssistantTurn(turn) => {
            let mut lines = Vec::new();
            lines.extend(
                turn.preface.iter().map(|entry| assistant_turn_entry_line_text(entry, true)),
            );
            lines.extend(
                turn.children.iter().map(|entry| assistant_turn_entry_line_text(entry, false)),
            );
            lines.push(assistant_line_text(turn.assistant));
            lines
        }
    }
}

fn message_line_text(message: &UiMessage) -> String {
    match message {
        UiMessage::User(message) => format!("USER   {}", message.text),
        UiMessage::Assistant(message) => assistant_line_text(message),
        UiMessage::Step(step) => format!(
            "STEP   #{} {:?} {}",
            step.step_index,
            step.state,
            step.finish_reason.as_deref().unwrap_or("running")
        ),
        UiMessage::System(message) => format!("SYSTEM {}", message.text),
        UiMessage::ToolCall(message) => format!("TOOL   {} {:?}", message.tool_name, message.state),
        UiMessage::ToolResult(message) => format!("RESULT {}", message.content),
        UiMessage::Thinking(message) => {
            format!("THINK  {}", message.summary.as_deref().unwrap_or(message.content.as_str()))
        }
        UiMessage::Error(message) => format!("ERROR  {}", message.message),
    }
}

fn assistant_line_text(message: &UiAssistantMessage) -> String {
    format!("ASSIST {} [{}]", message.text, terminal_label_text(&message.terminal))
}

fn assistant_turn_entry_line_text(entry: &TuiAssistantTurnEntry<'_>, is_preface: bool) -> String {
    let phase = if is_preface { "PRE" } else { "SUB" };
    match entry {
        TuiAssistantTurnEntry::Thinking(message) => format!(
            "{phase} THINK {}",
            message.summary.as_deref().unwrap_or(message.content.as_str())
        ),
        TuiAssistantTurnEntry::Step(step) => format!(
            "{phase} STEP  #{} {:?} {}",
            step.step_index,
            step.state,
            step.finish_reason.as_deref().unwrap_or("running")
        ),
        TuiAssistantTurnEntry::Tool(tool_group) => format!(
            "{phase} TOOL  {} {:?} [{} result{}]",
            tool_group.call.tool_name,
            tool_group.call.state,
            tool_group.results.len(),
            if tool_group.results.len() == 1 { "" } else { "s" }
        ),
        TuiAssistantTurnEntry::ToolResult(message) => {
            format!("{phase} RESULT {}", message.content)
        }
        TuiAssistantTurnEntry::CollapsedTools(batch) => format!(
            "{phase} TOOLS {} [{} result{}]",
            batch.summary,
            batch.total_results,
            if batch.total_results == 1 { "" } else { "s" }
        ),
    }
}

fn wrapped_line_count(text: &str, content_width: u16) -> usize {
    let content_width = content_width.max(1) as usize;
    let mut total_rows = 0usize;

    for segment in text.split('\n') {
        let display_width = segment.chars().map(|ch| ch.width().unwrap_or(0)).sum::<usize>();
        total_rows = total_rows.saturating_add(display_width.max(1).div_ceil(content_width));
    }

    total_rows.max(1)
}

fn terminal_label_text(terminal: &UiTurnTerminal) -> &'static str {
    match terminal {
        UiTurnTerminal::Pending => "pending",
        UiTurnTerminal::Streaming => "streaming",
        UiTurnTerminal::Done { .. } => "done",
        UiTurnTerminal::Cancelled { .. } => "cancelled",
        UiTurnTerminal::TimedOut { .. } => "timeout",
        UiTurnTerminal::Error { .. } => "error",
    }
}

fn excerpt_around(text: &str, start: usize, end: usize) -> String {
    let preview_start = floor_char_boundary(text, start.saturating_sub(24));
    let preview_end = ceil_char_boundary(text, end.saturating_add(24));
    let prefix = if preview_start > 0 { "..." } else { "" };
    let suffix = if preview_end < text.len() { "..." } else { "" };
    // 用 […] 标记匹配关键词，便于在预览中直观定位
    let matched = &text[start..end];
    format!("{prefix}{}[{matched}]{}{suffix}", &text[preview_start..start], &text[end..preview_end],)
}

fn floor_char_boundary(text: &str, index: usize) -> usize {
    let mut index = index.min(text.len());
    while index > 0 && !text.is_char_boundary(index) {
        index = index.saturating_sub(1);
    }
    index
}

fn ceil_char_boundary(text: &str, index: usize) -> usize {
    let mut index = index.min(text.len());
    while index < text.len() && !text.is_char_boundary(index) {
        index = index.saturating_add(1);
    }
    index
}

fn add_usage(target: &mut UiTokenUsage, usage: &UiTokenUsage) {
    target.input_tokens = target.input_tokens.saturating_add(usage.input_tokens);
    target.output_tokens = target.output_tokens.saturating_add(usage.output_tokens);
    target.cached_tokens = target.cached_tokens.saturating_add(usage.cached_tokens);
    target.reasoning_tokens = target.reasoning_tokens.saturating_add(usage.reasoning_tokens);
}
#[cfg(test)]
#[path = "selectors_tests.rs"]
mod selectors_tests;
