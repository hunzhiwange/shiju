//! TUI 的派生状态入口。
//!
//! 当前 selectors 只依赖 `TuiState` 与内部模型，不依赖 renderer，
//! 这样后续无论是 Ratatui、snapshot 测试还是 shadow compare，都可以复用相同的
//! 最小计算逻辑。

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::mem;

use super::TuiState;
use cli_interaction_model::{
    UiAssistantMessage, UiMessage, UiMessageBase, UiMessageId, UiProgressMessage, UiSearchMatch,
    UiStep, UiSystemMessage, UiThinkingBlock, UiTokenUsage, UiToolCall, UiToolResult, UiTurnTerminal,
};
use unicode_width::UnicodeWidthChar;

/// 状态线可直接消费的汇总结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TuiStatusSummary {
    pub session_id: Option<String>,
    pub title: String,
    pub provider_name: Option<String>,
    pub model_name: Option<String>,
    pub message_count: usize,
    pub assistant_message_count: usize,
    pub step_count: usize,
    pub pending_questions: usize,
    pub todo_count: usize,
    pub overlay_depth: usize,
    pub prompt_busy: bool,
    pub turn_terminal: UiTurnTerminal,
    pub token_usage: UiTokenUsage,
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
    pub(crate) progress: Vec<&'a UiProgressMessage>,
    pub(crate) notices: Vec<&'a UiSystemMessage>,
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
    pub(crate) total_progress: usize,
    pub(crate) notice_count: usize,
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
    verbose: bool,
    item_keys: Vec<String>,
    item_heights: Vec<usize>,
    measured_keys: BTreeSet<String>,
    prefix_rows: Vec<usize>,
}

impl TuiTranscriptLayoutCacheBucket {
    fn rebuild_prefix_from(&mut self, start: usize) {
        if self.prefix_rows.len() != self.item_heights.len().saturating_add(1) {
            self.prefix_rows = vec![0; self.item_heights.len().saturating_add(1)];
        }
        let start = start.min(self.item_heights.len());
        if start == 0 {
            self.prefix_rows[0] = 0;
        }
        for index in start..self.item_heights.len() {
            self.prefix_rows[index + 1] =
                self.prefix_rows[index].saturating_add(self.item_heights[index].max(1));
        }
    }
}

/// 按宽度分组的 transcript wrap/height cache。
///
/// 每个 bucket 都只缓存某个 content width 下的顶层 transcript item 行高，
/// 让 scroll/visible window 在相同宽度下复用估算结果，而不是每次重新 wrap 全段文本。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct TuiTranscriptLayoutCache {
    buckets: BTreeMap<(u16, bool), TuiTranscriptLayoutCacheBucket>,
}

const MAX_TRANSCRIPT_LAYOUT_BUCKETS: usize = 16;

impl TuiTranscriptLayoutCache {
    pub(crate) fn rebuild_width(
        &mut self,
        messages: &[UiMessage],
        projection: &TuiTranscriptProjectionCache,
        content_width: u16,
        verbose: bool,
    ) {
        if content_width == 0 {
            return;
        }

        let key = (content_width, verbose);
        if !self.buckets.contains_key(&key)
            && self.buckets.len() >= MAX_TRANSCRIPT_LAYOUT_BUCKETS
            && let Some(oldest_key) = self.buckets.keys().next().copied()
        {
            self.buckets.remove(&oldest_key);
        }
        self.buckets.entry((content_width, verbose)).or_insert_with(|| {
            derive_transcript_layout_cache_bucket(messages, projection, content_width, verbose)
        });
    }

    pub(crate) fn reconcile_projection(
        &mut self,
        messages: &[UiMessage],
        projection: &TuiTranscriptProjectionCache,
    ) {
        for bucket in self.buckets.values_mut() {
            let old_heights = bucket
                .item_keys
                .iter()
                .cloned()
                .zip(bucket.item_heights.iter().copied())
                .collect::<BTreeMap<_, _>>();
            bucket.item_keys = projection.keys().to_vec();
            bucket.item_heights = projection
                .items()
                .iter()
                .zip(projection.keys())
                .map(|(item, key)| {
                    old_heights.get(key).copied().unwrap_or_else(|| {
                        estimate_transcript_projection_item_height(
                            messages,
                            item,
                            bucket.content_width,
                            bucket.verbose,
                        )
                    })
                })
                .collect();
            bucket.measured_keys.retain(|key| projection.keys().contains(key));
            bucket.prefix_rows = vec![0; bucket.item_heights.len().saturating_add(1)];
            bucket.rebuild_prefix_from(0);
        }
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

        for bucket in self.buckets.values_mut() {
            if bucket.item_heights.len() != projection.len() {
                *bucket = derive_transcript_layout_cache_bucket(
                    messages,
                    projection,
                    bucket.content_width,
                    bucket.verbose,
                );
                continue;
            }

            let Some(item) = projection.item(item_index) else {
                continue;
            };
            bucket.item_heights[item_index] = estimate_transcript_projection_item_height(
                messages,
                item,
                bucket.content_width,
                bucket.verbose,
            );
            bucket.measured_keys.remove(&bucket.item_keys[item_index]);
            bucket.rebuild_prefix_from(item_index);
        }
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
    keys: Vec<String>,
}

impl TuiTranscriptProjectionCache {
    fn len(&self) -> usize {
        self.items.len()
    }

    fn items(&self) -> &[TuiTranscriptProjectionItem] {
        &self.items
    }

    pub(crate) fn anchors(&self) -> &[usize] {
        &self.anchors
    }

    pub(crate) fn keys(&self) -> &[String] {
        &self.keys
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
            | UiMessage::ToolResult(_)
            | UiMessage::Progress(_) => {
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
            UiMessage::System(system) if is_tool_notice(system) => {
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
            UiMessage::System(_)
            | UiMessage::Error(_)
            | UiMessage::Attachment(_)
            | UiMessage::CompactBoundary(_)
            | UiMessage::Unknown(_) => {
                flush_pending_preface(&mut items, &mut pending_preface);
                items.push(TranscriptBuilderItem::Standalone(indexed_message));
                pending_user_id = None;
            }
        }
    }

    flush_pending_preface(&mut items, &mut pending_preface);

    let items = items.into_iter().map(finalize_transcript_projection_item).collect::<Vec<_>>();
    let anchors = items.iter().map(|item| item.span().start).collect();
    let keys = items.iter().map(|item| transcript_projection_key(messages, item)).collect();
    TuiTranscriptProjectionCache { items, anchors, keys }
}

/// 汇总当前状态线关心的统计信息。
pub fn select_status_summary(state: &TuiState) -> TuiStatusSummary {
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

fn transcript_projection_key(messages: &[UiMessage], item: &TuiTranscriptProjectionItem) -> String {
    let span = item.span();
    let member_ids = messages[span.start.min(messages.len())..span.end.min(messages.len())]
        .iter()
        .map(|message| message.id().as_str())
        .collect::<Vec<_>>()
        .join("|");
    match item {
        TuiTranscriptProjectionItem::Standalone { .. } => member_ids,
        TuiTranscriptProjectionItem::AssistantTurn { .. } => format!("group:{member_ids}"),
    }
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
            UiMessage::ToolCall(call) => {
                entries.push(TuiAssistantTurnEntry::Tool(TuiToolCallGroup {
                    call,
                    results: Vec::new(),
                    progress: Vec::new(),
                    notices: Vec::new(),
                }));
            }
            UiMessage::ToolResult(result) => {
                if let Some(tool_group) = entries.iter_mut().rev().find_map(|entry| match entry {
                    TuiAssistantTurnEntry::Tool(tool_group)
                        if tool_result_matches_call(tool_group.call, result) =>
                    {
                        Some(tool_group)
                    }
                    _ => None,
                }) {
                    tool_group.results.push(result);
                    continue;
                }
                entries.push(TuiAssistantTurnEntry::ToolResult(result));
            }
            UiMessage::Progress(progress) => {
                if let Some(tool_group) = entries.iter_mut().rev().find_map(|entry| match entry {
                    TuiAssistantTurnEntry::Tool(tool_group)
                        if tool_aux_matches_call(&progress.base, tool_group.call) =>
                    {
                        Some(tool_group)
                    }
                    _ => None,
                }) {
                    tool_group.progress.push(progress);
                }
            }
            UiMessage::System(notice) if is_tool_notice(notice) => {
                if let Some(tool_group) = entries.iter_mut().rev().find_map(|entry| match entry {
                    TuiAssistantTurnEntry::Tool(tool_group)
                        if tool_aux_matches_call(&notice.base, tool_group.call) =>
                    {
                        Some(tool_group)
                    }
                    _ => None,
                }) {
                    tool_group.notices.push(notice);
                }
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
    let mut total_progress = 0usize;
    let mut notice_count = 0usize;

    for call in &calls {
        let Some(kind) = collapse_kind_for_tool_group(call) else {
            continue;
        };

        total_results = total_results.saturating_add(call.results.len());
        total_progress = total_progress.saturating_add(call.progress.len());
        notice_count = notice_count.saturating_add(call.notices.len());
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
        total_progress,
        notice_count,
    }
}

fn assistant_turn_accepts_message(turn: &AssistantTurnBuilder<'_>, message: &UiMessage) -> bool {
    match message {
        UiMessage::Thinking(_) | UiMessage::Step(_) | UiMessage::ToolCall(_) => true,
        UiMessage::ToolResult(result) => {
            tool_result_matches_messages(result, &turn.child_messages)
                || tool_result_matches_messages(result, &turn.preface_messages)
        }
        UiMessage::Progress(progress) => {
            tool_aux_matches_messages(&progress.base, &turn.child_messages)
                || tool_aux_matches_messages(&progress.base, &turn.preface_messages)
        }
        UiMessage::System(notice) if is_tool_notice(notice) => {
            tool_aux_matches_messages(&notice.base, &turn.child_messages)
                || tool_aux_matches_messages(&notice.base, &turn.preface_messages)
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
        UiMessage::Progress(progress) => tool_aux_matches_messages(&progress.base, pending_preface),
        UiMessage::System(notice) if is_tool_notice(notice) => {
            tool_aux_matches_messages(&notice.base, pending_preface)
        }
        _ => false,
    }
}

fn tool_aux_matches_messages(base: &UiMessageBase, messages: &[IndexedUiMessage<'_>]) -> bool {
    messages.iter().rev().any(|message| match message {
        IndexedUiMessage { message: UiMessage::ToolCall(call), .. } => {
            tool_aux_matches_call(base, call)
        }
        _ => false,
    })
}

fn tool_aux_matches_call(base: &UiMessageBase, call: &UiToolCall) -> bool {
    base.parent_id.as_ref().is_some_and(|parent_id| parent_id == &call.base.id)
}

fn is_tool_notice(message: &UiSystemMessage) -> bool {
    let text = message.text.trim_start().to_ascii_lowercase();
    message.base.parent_id.is_some()
        && ["hook:", "background:", "teammate shutdown:"]
            .iter()
            .any(|prefix| text.starts_with(prefix))
}

fn tool_result_matches_messages(result: &UiToolResult, messages: &[IndexedUiMessage<'_>]) -> bool {
    messages.iter().rev().any(|message| match message {
        IndexedUiMessage { message: UiMessage::ToolCall(call), .. } => {
            tool_result_matches_call(call, result)
        }
        _ => false,
    })
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
            .unwrap_or(Cow::Borrowed(message.tool_name.as_str())),
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
        UiMessage::Attachment(message) => Cow::Owned(format!(
            "{} {} {} {}",
            message.name,
            message.media_type.as_deref().unwrap_or(""),
            message.uri,
            message.alt_text.as_deref().unwrap_or("")
        )),
        UiMessage::Progress(message) => Cow::Borrowed(message.label.as_str()),
        UiMessage::CompactBoundary(message) => Cow::Borrowed(message.summary.as_str()),
        UiMessage::Unknown(message) => {
            Cow::Owned(format!("{} {}", message.external_kind, message.summary))
        }
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
    verbose: bool,
) -> TuiTranscriptLayoutCacheBucket {
    let item_heights = projection
        .items()
        .iter()
        .map(|item| {
            estimate_transcript_projection_item_height(messages, item, content_width, verbose)
        })
        .collect();
    let mut bucket = TuiTranscriptLayoutCacheBucket {
        content_width,
        verbose,
        item_keys: projection.keys().to_vec(),
        item_heights,
        measured_keys: BTreeSet::new(),
        prefix_rows: vec![0; projection.len().saturating_add(1)],
    };
    bucket.rebuild_prefix_from(0);
    bucket
}

fn estimate_transcript_projection_item_height(
    messages: &[UiMessage],
    item: &TuiTranscriptProjectionItem,
    content_width: u16,
    verbose: bool,
) -> usize {
    let item = item.materialize(messages);
    estimate_transcript_item_height(&item, content_width, verbose)
}

fn estimate_transcript_item_height(
    item: &TuiTranscriptItem<'_>,
    content_width: u16,
    verbose: bool,
) -> usize {
    transcript_item_line_texts(item, verbose)
        .into_iter()
        .map(|line| wrapped_line_count(line.as_str(), content_width))
        .sum::<usize>()
        .saturating_add(1)
}

fn transcript_item_line_texts(item: &TuiTranscriptItem<'_>, verbose: bool) -> Vec<String> {
    match item {
        TuiTranscriptItem::Standalone(UiMessage::System(message))
            if message.level == cli_interaction_model::UiSystemMessageLevel::Info =>
        {
            Vec::new()
        }
        TuiTranscriptItem::Standalone(UiMessage::Assistant(message))
            if message.text.trim().is_empty() =>
        {
            Vec::new()
        }
        TuiTranscriptItem::Standalone(message) => vec![message_line_text(message)],
        TuiTranscriptItem::AssistantTurn(turn) => {
            let mut lines = Vec::new();
            lines.extend(
                turn.preface
                    .iter()
                    .flat_map(|entry| assistant_turn_entry_line_texts(entry, true, verbose)),
            );
            lines.extend(
                turn.children
                    .iter()
                    .flat_map(|entry| assistant_turn_entry_line_texts(entry, false, verbose)),
            );
            if !turn.assistant.text.trim().is_empty() {
                lines.push(assistant_line_text(turn.assistant));
            }
            lines
        }
    }
}

fn message_line_text(message: &UiMessage) -> String {
    match message {
        UiMessage::User(message) => format!("❯ {}", message.text),
        UiMessage::Assistant(message) => assistant_line_text(message),
        UiMessage::Step(step) => format!(
            "STEP   #{} {:?} {}",
            step.step_index,
            step.state,
            step.finish_reason.as_deref().unwrap_or("running")
        ),
        UiMessage::System(message) => format!("● {}", message.text),
        UiMessage::ToolCall(message) => format!("TOOL   {} {:?}", message.tool_name, message.state),
        UiMessage::ToolResult(message) => format!("RESULT {}", message.content),
        UiMessage::Thinking(message) => {
            format!("THINK  {}", message.summary.as_deref().unwrap_or(message.content.as_str()))
        }
        UiMessage::Error(message) => format!("● {}", message.message),
        UiMessage::Attachment(message) => format!("ATTACH {}", message.name),
        UiMessage::Progress(message) => format!("PROGRESS {}", message.label),
        UiMessage::CompactBoundary(_) => {
            "✻ Conversation compacted (Ctrl+O for history)".to_string()
        }
        UiMessage::Unknown(message) => format!("● {}", message.summary),
    }
}

fn assistant_line_text(message: &UiAssistantMessage) -> String {
    format!("● {}", message.text)
}

fn assistant_turn_entry_line_texts(
    entry: &TuiAssistantTurnEntry<'_>,
    is_preface: bool,
    verbose: bool,
) -> Vec<String> {
    let phase = if is_preface { "PRE" } else { "SUB" };
    let summary = match entry {
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
    };
    if verbose && let TuiAssistantTurnEntry::CollapsedTools(batch) = entry {
        let mut lines = vec![summary];
        for call in &batch.calls {
            lines.push(format!("{phase} TOOL {}", call.call.tool_name));
            lines.extend(call.results.iter().map(|result| result.content.clone()));
            lines.extend(call.progress.iter().map(|progress| progress.label.clone()));
            lines.extend(call.notices.iter().map(|notice| notice.text.clone()));
        }
        lines
    } else {
        vec![summary]
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

fn excerpt_around(text: &str, start: usize, end: usize) -> String {
    let preview_start = floor_char_boundary(text, start.saturating_sub(24));
    let preview_end = ceil_char_boundary(text, end.saturating_add(24));
    let prefix = if preview_start > 0 { "..." } else { "" };
    let suffix = if preview_end < text.len() { "..." } else { "" };
    // 用 […] 标记匹配关键词，便于在预览中直观定位
    let matched = &text[start..end];
    format!("{prefix}{}[{matched}]{}{suffix}", &text[preview_start..start], &text[end..preview_end])
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
