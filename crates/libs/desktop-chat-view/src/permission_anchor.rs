use session_ui_types as models;
use desktop_chat_tools::tools::tool_parse::{is_explore_tool,tool_identity_from_raw,tool_name_from_raw};
use desktop_chat_tools::tools::tool_permission::{pending_permission_targets_message,pending_permission_targets_tool_call};
pub fn permission_target_tool_anchor_fraction(
    message: &models::ChatMessage,
    request: Option<&gateway_interaction::PendingPermissionRequestDto>,
    message_id: Option<&str>,
    blocks: &[crate::message_view::ParsedChatBlock],
    tool_last: &std::collections::HashMap<String, usize>,
) -> Option<f32> {
    if !pending_permission_targets_message(request, message_id) {
        return None;
    }

    if message.role == models::ChatRole::Tool {
        return Some(0.30);
    }

    if message.role != models::ChatRole::Assistant {
        return None;
    }

    let mut visible_tools: Vec<Vec<usize>> = Vec::new();
    let mut pending_explore_group: Vec<usize> = Vec::new();

    for (block_idx, block) in blocks.iter().enumerate() {
        let crate::message_view::ParsedChatBlock::Tool { raw } = block else {
            if !pending_explore_group.is_empty() {
                visible_tools.push(std::mem::take(&mut pending_explore_group));
            }
            continue;
        };

        let Some(tool_name) = tool_name_from_raw(raw) else {
            if !pending_explore_group.is_empty() {
                visible_tools.push(std::mem::take(&mut pending_explore_group));
            }
            continue;
        };
        if is_explore_tool(&tool_name) {
            pending_explore_group.push(block_idx);
            continue;
        }

        if !pending_explore_group.is_empty() {
            visible_tools.push(std::mem::take(&mut pending_explore_group));
        }
        if let Some(identity) = tool_identity_from_raw(raw)
            && tool_last.get(&identity).copied() != Some(block_idx)
        {
            continue;
        }

        visible_tools.push(vec![block_idx]);
    }

    if !pending_explore_group.is_empty() {
        visible_tools.push(pending_explore_group);
    }

    if visible_tools.is_empty() {
        return Some(0.50);
    }

    let target_tool_idx = visible_tools.iter().position(|group| {
        group.iter().any(|block_idx| {
            let Some(crate::message_view::ParsedChatBlock::Tool { raw }) = blocks.get(*block_idx)
            else {
                return false;
            };
            pending_permission_targets_tool_call(request, message_id, raw)
        })
    })?;
    let step = 1.0 / (visible_tools.len() as f32 + 1.0);
    Some((step * (target_tool_idx as f32 + 1.0)).clamp(0.22, 0.78))
}
