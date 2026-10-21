//! 助手消息渲染缓存构建。
//!
//! 该模块负责根据解析后的消息块构建渲染缓存，并在流式场景下提供
//! 缓存新鲜度判断与即时回退能力。

use std::borrow::Cow;

use crate::app::App;
use crate::app::models::{self, ParsedChatBlock};
use crate::app::session::chat_timeline::{ChatEntryKey, ChatEntryRevision};

use super::super::tools::{is_explore_tool, tool_identity_from_raw, tool_name_from_raw};
use super::super::utils::{normalize_display_text, strip_internal_tool_trace, truncate_chars};
use super::parse::{RenderBlock, borrowed_blocks, hash_chat_content, owned_blocks_from_raw};
use super::text::estimate_text_height;
use super::tool_summaries::{
    collect_tool_card_texts, count_code_blocks, normalized_visible_text, summarize_explore_items,
};

const LARGE_MESSAGE_RAW_CHARS: usize = 12_000;
const RENDER_WIDTH_BUCKET_PX: f32 = 32.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RevisionRenderKey {
    entry_key: ChatEntryKey,
    revision: ChatEntryRevision,
    width_bucket: u32,
    dark_theme: bool,
    show_reasoning_summary: bool,
}

impl RevisionRenderKey {
    pub(crate) fn new(
        entry_key: ChatEntryKey,
        revision: ChatEntryRevision,
        width: f32,
        dark_theme: bool,
        show_reasoning_summary: bool,
    ) -> Self {
        Self {
            entry_key,
            revision,
            width_bucket: width_bucket(width),
            dark_theme,
            show_reasoning_summary,
        }
    }

    #[cfg(test)]
    pub(crate) fn matches(
        &self,
        entry_key: &ChatEntryKey,
        revision: ChatEntryRevision,
        width: f32,
        dark_theme: bool,
        show_reasoning_summary: bool,
    ) -> bool {
        self.entry_key == *entry_key
            && self.revision == revision
            && self.width_bucket == width_bucket(width)
            && self.dark_theme == dark_theme
            && self.show_reasoning_summary == show_reasoning_summary
    }

    pub(crate) fn matches_key(&self, other: &Self) -> bool {
        self == other
    }
}

fn width_bucket(width: f32) -> u32 {
    (width.max(0.0) / RENDER_WIDTH_BUCKET_PX).floor() as u32
}
const LARGE_MESSAGE_VISIBLE_CHARS: usize = 8_000;
const LARGE_MESSAGE_BLOCKS: usize = 12;
const LARGE_MESSAGE_LINES: usize = 220;
const LARGE_MESSAGE_TOOL_BLOCKS: usize = 10;
const LARGE_MESSAGE_CODE_BLOCKS: usize = 6;

fn flush_explore_summaries(
    out: &mut Vec<(usize, String)>,
    items: &mut Vec<String>,
    group_idx: usize,
    force_running: bool,
) {
    if let Some(summary) =
        summarize_explore_items(items.iter().map(String::as_str), group_idx, force_running)
    {
        out.push(summary);
    }
    items.clear();
}

pub(crate) fn build_render_cache_entry(
    raw: &str,
    visible_for_copy: &str,
    copy_hash: u64,
    show_reasoning_summary: bool,
) -> models::ChatRenderCacheEntry {
    #[cfg(test)]
    super::super::performance::record_full_content_parse(raw.len());
    const FOLD_PREVIEW_CHARS: usize = 2400;
    const FOLDABLE_VISIBLE_CHARS: usize = 6000;
    const FOLDABLE_TOTAL_CHARS: usize = 9000;
    const FOLDABLE_BLOCKS: usize = 18;

    let blocks = owned_blocks_from_raw(raw);
    let tool_block_metadata = blocks
        .iter()
        .map(|block| match block {
            ParsedChatBlock::Tool { raw } => Some(models::ToolBlockRenderMetadata {
                name: tool_name_from_raw(raw),
                identity: tool_identity_from_raw(raw),
            }),
            _ => None,
        })
        .collect::<Vec<_>>();
    let has_special_blocks = blocks
        .iter()
        .any(|block| matches!(block, ParsedChatBlock::Think { .. } | ParsedChatBlock::Tool { .. }));

    let mut special_text_blocks = Vec::new();
    let mut tool_card_text_blocks = Vec::new();
    let mut explore_summary_text_blocks = Vec::new();
    let mut explore_items: Vec<String> = Vec::new();
    let mut group_idx = 0usize;
    let mut estimated_expanded_height = 40.0;
    let mut explore_group_force_running = false;

    for (block_idx, block) in borrowed_blocks(&blocks).enumerate() {
        match block {
            RenderBlock::Think { content, open } => {
                if open || show_reasoning_summary {
                    if open {
                        explore_group_force_running = true;
                    }
                    flush_explore_summaries(
                        &mut explore_summary_text_blocks,
                        &mut explore_items,
                        group_idx,
                        explore_group_force_running,
                    );
                    explore_group_force_running = open;
                    group_idx = group_idx.saturating_add(1);
                    let normalized = normalize_display_text(content.trim()).into_owned();
                    estimated_expanded_height +=
                        estimate_text_height(&normalized).min(160.0) + 44.0;
                }
            }
            RenderBlock::Tool { raw } => {
                if let Some(name) = tool_block_metadata
                    .get(block_idx)
                    .and_then(Option::as_ref)
                    .and_then(|metadata| metadata.name.as_deref())
                    && is_explore_tool(&name)
                {
                    explore_items.push(raw.to_string());
                    estimated_expanded_height += 28.0;
                    continue;
                }

                flush_explore_summaries(
                    &mut explore_summary_text_blocks,
                    &mut explore_items,
                    group_idx,
                    explore_group_force_running,
                );
                explore_group_force_running = false;
                group_idx = group_idx.saturating_add(1);
                estimated_expanded_height += 72.0;

                if let Some(tool_texts) = collect_tool_card_texts(raw) {
                    tool_card_text_blocks.push(tool_texts);
                }
            }
            RenderBlock::Text { content } => {
                let Some(normalized) = normalized_visible_text(content) else {
                    continue;
                };

                flush_explore_summaries(
                    &mut explore_summary_text_blocks,
                    &mut explore_items,
                    group_idx,
                    explore_group_force_running,
                );
                explore_group_force_running = false;
                group_idx = group_idx.saturating_add(1);
                estimated_expanded_height += estimate_text_height(&normalized) + 28.0;
                special_text_blocks.push(normalized);
            }
        }
    }

    flush_explore_summaries(
        &mut explore_summary_text_blocks,
        &mut explore_items,
        group_idx,
        explore_group_force_running,
    );

    let stripped_display =
        normalize_display_text(strip_internal_tool_trace(visible_for_copy).trim()).into_owned();
    let display_text = if stripped_display.is_empty() {
        tool_card_text_blocks
            .iter()
            .flatten()
            .find(|text| !text.trim().is_empty())
            .cloned()
            .or_else(|| {
                blocks.iter().find_map(|block| match block {
                    ParsedChatBlock::Tool { raw } => collect_tool_card_texts(raw)
                        .and_then(|texts| texts.into_iter().find(|text| !text.trim().is_empty())),
                    _ => None,
                })
            })
            .unwrap_or_default()
    } else {
        stripped_display
    };
    let normalized_preview = display_text.clone();
    let preview_text = if normalized_preview.len() > FOLD_PREVIEW_CHARS {
        truncate_chars(&normalized_preview, FOLD_PREVIEW_CHARS).to_string()
    } else {
        normalized_preview.clone()
    };
    let line_count = raw.lines().count().max(normalized_preview.lines().count());
    let tool_block_count =
        blocks.iter().filter(|block| matches!(block, ParsedChatBlock::Tool { .. })).count();
    let code_block_count = count_code_blocks(&normalized_preview);
    let is_large_message = raw.len() >= LARGE_MESSAGE_RAW_CHARS
        || normalized_preview.len() >= LARGE_MESSAGE_VISIBLE_CHARS
        || blocks.len() >= LARGE_MESSAGE_BLOCKS
        || line_count >= LARGE_MESSAGE_LINES
        || tool_block_count >= LARGE_MESSAGE_TOOL_BLOCKS
        || code_block_count >= LARGE_MESSAGE_CODE_BLOCKS;
    let foldable = raw.len() >= FOLDABLE_TOTAL_CHARS
        || normalized_preview.len() >= FOLDABLE_VISIBLE_CHARS
        || blocks.len() >= FOLDABLE_BLOCKS
        || is_large_message;
    let estimated_collapsed_height =
        (estimate_text_height(&preview_text) + 96.0).clamp(88.0, 420.0);

    #[cfg(test)]
    super::super::performance::record_full_content_hash(raw.len());
    let content_hash = hash_chat_content(raw);

    models::ChatRenderCacheEntry {
        revision_key: None,
        content_hash,
        show_reasoning_summary,
        copy_content_hash: Some(copy_hash),
        blocks,
        tool_block_metadata,
        has_special_blocks,
        special_text_blocks,
        tool_card_text_blocks,
        explore_summary_text_blocks,
        display_text,
        preview_text,
        foldable,
        is_large_message,
        estimated_collapsed_height,
        estimated_expanded_height: estimated_expanded_height.max(estimated_collapsed_height),
    }
}

pub(super) fn resolve_visible_text_and_copy_hash<'a>(
    app: &'a App,
    idx: usize,
    content: &'a str,
    current_content_hash: u64,
    render_cache: &models::ChatRenderCacheEntry,
) -> (Cow<'a, str>, u64) {
    let cache_is_fresh = render_cache.content_hash == current_content_hash;

    if cache_is_fresh
        && let (Some(visible_text), Some(copy_hash)) = (
            app.chat_visible_text_cache.get(idx).and_then(|text| text.as_deref()),
            app.chat_copy_hash_cache.get(idx).and_then(|hash| *hash),
        )
    {
        return (Cow::Borrowed(visible_text), copy_hash);
    }

    let (_, visible_text, _) = crate::app::ui::chat::split_think(content);
    #[cfg(test)]
    super::super::performance::record_full_content_hash(visible_text.len());
    let copy_hash = hash_chat_content(&visible_text);
    (Cow::Owned(visible_text), copy_hash)
}

pub(crate) fn effective_assistant_render_cache<'a>(
    content: &str,
    current_content_hash: u64,
    render_cache: &'a models::ChatRenderCacheEntry,
    visible_for_copy: &str,
    copy_hash: u64,
    _is_streaming_assistant: bool,
    show_reasoning_summary: bool,
) -> Cow<'a, models::ChatRenderCacheEntry> {
    if render_cache.content_hash != current_content_hash
        || render_cache.show_reasoning_summary != show_reasoning_summary
    {
        Cow::Owned(build_render_cache_entry(
            content,
            visible_for_copy,
            copy_hash,
            show_reasoning_summary,
        ))
    } else {
        Cow::Borrowed(render_cache)
    }
}

pub(crate) fn assistant_render_blocks<'a>(
    content: &str,
    current_content_hash: u64,
    render_cache: &'a models::ChatRenderCacheEntry,
    _is_streaming_assistant: bool,
) -> (Cow<'a, [ParsedChatBlock]>, bool) {
    if render_cache.content_hash != current_content_hash {
        #[cfg(test)]
        super::super::performance::record_full_content_parse(content.len());
        let blocks = owned_blocks_from_raw(content);
        let has_special_blocks =
            blocks.iter().any(|block| !matches!(block, ParsedChatBlock::Text { .. }));
        (Cow::Owned(blocks), has_special_blocks)
    } else {
        (Cow::Borrowed(render_cache.blocks.as_slice()), render_cache.has_special_blocks)
    }
}
