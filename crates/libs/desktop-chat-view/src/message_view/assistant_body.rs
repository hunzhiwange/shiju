//! 助手工具块的稳定去重，原始内容与缓存元数据共用同一入口。
use super::ParsedChatBlock;
use crate::render_cache::ToolBlockRenderMetadata;
use desktop_chat_tools::tools::tool_parse::{
    is_explore_tool, tool_identity_from_raw, tool_name_from_raw,
};
use std::collections::HashMap;
pub fn deduped_tool_last_indices(blocks: &[ParsedChatBlock]) -> HashMap<String, usize> {
    deduped_tool_last_indices_with_metadata(blocks, &[])
}

pub fn deduped_tool_last_indices_with_metadata(
    blocks: &[ParsedChatBlock],
    tool_block_metadata: &[Option<ToolBlockRenderMetadata>],
) -> HashMap<String, usize> {
    let mut tool_last: HashMap<String, usize> = HashMap::new();

    for (block_idx, block) in blocks.iter().enumerate() {
        let ParsedChatBlock::Tool { raw } = block else {
            continue;
        };

        let metadata = tool_block_metadata.get(block_idx).and_then(Option::as_ref);
        let name =
            metadata.and_then(|metadata| metadata.name.clone()).or_else(|| tool_name_from_raw(raw));
        if let Some(name) = name
            && is_explore_tool(&name)
        {
            continue;
        }

        let identity = metadata
            .and_then(|metadata| metadata.identity.clone())
            .or_else(|| tool_identity_from_raw(raw));
        if let Some(identity) = identity {
            tool_last.insert(identity, block_idx);
        }
    }

    tool_last
}

/// Host-owned renderers consume this sequence; grouping and block indices stay here.
pub enum BodyPart {
    Think {
        block_idx: usize,
        think_idx: usize,
    },
    Tool {
        block_idx: usize,
        tool_idx: usize,
    },
    Text {
        block_idx: usize,
        text_idx: usize,
    },
    Commands(Vec<desktop_chat_tools::tools::types::CommandItem>),
    Explore {
        group_idx: usize,
        items: Vec<desktop_chat_tools::tools::types::ExploreItem>,
        force_running: bool,
        closed: bool,
    },
}

/// Facts supplied by the owning text, reasoning and permission surfaces.
#[derive(Default)]
pub struct BlockPresentation {
    pub show_think: bool,
    pub think_running: bool,
    pub visible_text: bool,
    pub hide_after_explore: bool,
    pub group_command: bool,
}

pub fn assistant_body_sequence(
    blocks: &[ParsedChatBlock],
    metadata: &[Option<ToolBlockRenderMetadata>],
    presentation: &[BlockPresentation],
    text_count: usize,
) -> Vec<BodyPart> {
    use desktop_chat_tools::tools::types::{CommandItem, ExploreItem};
    let last = deduped_tool_last_indices_with_metadata(blocks, metadata);
    let mut result = Vec::new();
    let (mut think_idx, mut tool_idx, mut text_idx, mut group_idx) = (0, 0, 0, 0usize);
    let mut commands = Vec::new();
    let mut explore = Vec::new();
    let mut force_running = false;
    fn flush(result: &mut Vec<BodyPart>, commands: &mut Vec<CommandItem>) {
        // Keep empty flushes: host rendering retains the original call order.
        result.push(BodyPart::Commands(std::mem::take(commands)));
    }
    fn close(
        result: &mut Vec<BodyPart>,
        group_idx: &mut usize,
        explore: &mut Vec<ExploreItem>,
        force_running: &mut bool,
    ) {
        result.push(BodyPart::Explore {
            group_idx: *group_idx,
            items: std::mem::take(explore),
            force_running: *force_running,
            closed: true,
        });
        *force_running = false;
        *group_idx = group_idx.saturating_add(1);
    }
    for (block_idx, block) in blocks.iter().enumerate() {
        let fallback = BlockPresentation::default();
        let facts = presentation.get(block_idx).unwrap_or(&fallback);
        match block {
            ParsedChatBlock::Think { .. } => {
                if facts.show_think {
                    flush(&mut result, &mut commands);
                    close(&mut result, &mut group_idx, &mut explore, &mut force_running);
                    result.push(BodyPart::Think { block_idx, think_idx });
                } else if facts.think_running {
                    force_running = true;
                }
                think_idx += 1;
            }
            ParsedChatBlock::Tool { raw } => {
                let meta = metadata.get(block_idx).and_then(Option::as_ref);
                let name = meta.and_then(|m| m.name.clone()).or_else(|| tool_name_from_raw(raw));
                let is_explore = name.as_deref().is_some_and(is_explore_tool);
                let identity =
                    meta.and_then(|m| m.identity.clone()).or_else(|| tool_identity_from_raw(raw));
                if !is_explore
                    && identity.as_ref().is_some_and(|id| last.get(id).copied() != Some(block_idx))
                {
                    tool_idx += 1;
                    continue;
                }
                if is_explore {
                    flush(&mut result, &mut commands);
                    explore.push(ExploreItem { tool_idx, raw: raw.clone() });
                    tool_idx += 1;
                    continue;
                }
                let had_explore = !explore.is_empty();
                close(&mut result, &mut group_idx, &mut explore, &mut force_running);
                if !(had_explore && facts.hide_after_explore) {
                    if facts.group_command {
                        commands.push(CommandItem { tool_idx, raw: raw.clone() });
                    } else {
                        flush(&mut result, &mut commands);
                        result.push(BodyPart::Tool { block_idx, tool_idx });
                    }
                }
                tool_idx += 1;
            }
            ParsedChatBlock::Text { .. } => {
                if !facts.visible_text {
                    continue;
                }
                flush(&mut result, &mut commands);
                if text_idx >= text_count {
                    continue;
                }
                let current = text_idx;
                text_idx += 1;
                let had_explore = !explore.is_empty();
                close(&mut result, &mut group_idx, &mut explore, &mut force_running);
                if !(had_explore && facts.hide_after_explore) {
                    result.push(BodyPart::Text { block_idx, text_idx: current });
                }
            }
        }
    }
    flush(&mut result, &mut commands);
    result.push(BodyPart::Explore { group_idx, items: explore, force_running, closed: false });
    result
}

pub fn count_visible_non_explore_tool_cards(
    blocks: &[ParsedChatBlock],
    tool_block_metadata: &[Option<ToolBlockRenderMetadata>],
    tool_last: &HashMap<String, usize>,
    presentation: &[BlockPresentation],
) -> usize {
    let mut count = 0usize;
    let mut has_pending_explore_group = false;

    for (block_idx, block) in blocks.iter().enumerate() {
        match block {
            ParsedChatBlock::Think { .. } => {
                has_pending_explore_group = false;
            }
            ParsedChatBlock::Tool { raw } => {
                let metadata = tool_block_metadata.get(block_idx).and_then(Option::as_ref);
                let tool_name = metadata
                    .and_then(|metadata| metadata.name.clone())
                    .or_else(|| tool_name_from_raw(raw));
                let Some(tool_name) = tool_name else {
                    continue;
                };

                if is_explore_tool(&tool_name) {
                    has_pending_explore_group = true;
                    continue;
                }

                let identity = metadata
                    .and_then(|metadata| metadata.identity.clone())
                    .or_else(|| tool_identity_from_raw(raw));
                if let Some(identity) = identity
                    && tool_last.get(&identity).copied() != Some(block_idx)
                {
                    continue;
                }

                if has_pending_explore_group
                    && presentation.get(block_idx).is_some_and(|facts| facts.hide_after_explore)
                {
                    has_pending_explore_group = false;
                    continue;
                }

                has_pending_explore_group = false;
                count = count.saturating_add(1);
            }
            ParsedChatBlock::Text { .. } => {
                if presentation.get(block_idx).is_some_and(|facts| facts.visible_text) {
                    has_pending_explore_group = false;
                }
            }
        }
    }

    count
}
