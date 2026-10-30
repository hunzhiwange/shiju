//! Budget only the summary request copy. Stored history and checkpoint sources stay intact.

use crate::provider::provider::ModelLimit;
use serde_json::Value;

const TRUNCATED: &str =
    "\n[Content omitted to fit the summary request; original remains in chat history]\n";
const MIN_CONTENT_BYTES: usize = 256;

pub(super) fn input_budget(limit: &ModelLimit) -> u64 {
    let window = crate::session::processor::context_governance::compaction_input_window(limit);
    // Leave room for estimation/protocol overhead as well as the model's output allocation.
    (u128::from(window) * 95 / 100) as u64
}

pub(super) fn estimated_tokens(messages: &[Value]) -> u64 {
    messages.iter().fold(0_u64, |total, message| {
        let bytes = message.to_string().len() as u64;
        total.saturating_add(bytes.saturating_add(3) / 4)
    })
}

fn protected(message: &Value) -> bool {
    matches!(message["role"].as_str(), Some("system" | "developer"))
}

fn shorten_content(message: &mut Value) -> bool {
    let Some(content) = message.get("content") else { return false };
    let text = match content {
        Value::String(text) => text.clone(),
        // A tool's structured result can be represented by a textual excerpt without
        // modifying the call ID or arguments. Never rewrite multimodal user blocks.
        Value::Array(_) | Value::Object(_) if message["role"] == "tool" => content.to_string(),
        _ => return false,
    };
    if text.len() <= MIN_CONTENT_BYTES {
        return false;
    }
    let target = (text.len() / 2).max(MIN_CONTENT_BYTES);
    let excerpt_bytes = target.saturating_sub(TRUNCATED.len()) / 2;
    let mut head = excerpt_bytes.min(text.len());
    while !text.is_char_boundary(head) {
        head -= 1;
    }
    let mut tail = text.len().saturating_sub(excerpt_bytes);
    while !text.is_char_boundary(tail) {
        tail += 1;
    }
    message["content"] = Value::String(format!("{}{TRUNCATED}{}", &text[..head], &text[tail..]));
    true
}

fn shrink_largest_content(messages: &mut [Value], tools_only: bool) -> bool {
    let index = messages
        .iter()
        .enumerate()
        .filter(|(_, message)| !protected(message) && (!tools_only || message["role"] == "tool"))
        .filter_map(|(index, message)| {
            let content = message.get("content")?;
            let bytes = match content {
                Value::String(text) => text.len(),
                Value::Array(_) | Value::Object(_) if message["role"] == "tool" => {
                    content.to_string().len()
                }
                _ => return None,
            };
            (bytes > MIN_CONTENT_BYTES).then_some((index, bytes))
        })
        .max_by_key(|(_, bytes)| *bytes)
        .map(|(index, _)| index);
    index.is_some_and(|index| shorten_content(&mut messages[index]))
}

fn remove_oldest_group(messages: &mut Vec<Value>) -> bool {
    // The final item is the summary instruction, and at least one source item must remain.
    let source_len = messages.len().saturating_sub(1);
    let latest_user = messages[..source_len].iter().rposition(|item| item["role"] == "user");
    let Some(start) = messages[..source_len].iter().enumerate().position(|(index, item)| {
        !protected(item) && item["role"] != "tool" && Some(index) != latest_user
    }) else {
        return false;
    };
    let mut end = start + 1;
    while end < source_len && messages[end]["role"] == "tool" {
        end += 1;
    }
    if !messages[..source_len]
        .iter()
        .enumerate()
        .any(|(index, item)| !(start..end).contains(&index) && !protected(item))
    {
        return false;
    }
    messages.drain(start..end);
    true
}

/// Return whether any source content was omitted. The last item is always the summary prompt.
pub(super) fn fit(messages: &mut Vec<Value>, budget: u64) -> Result<bool, String> {
    let mut changed = false;
    while estimated_tokens(messages) > budget {
        let source_len = messages.len().saturating_sub(1);
        if shrink_largest_content(&mut messages[..source_len], true)
            || remove_oldest_group(messages)
            || shrink_largest_content(&mut messages[..source_len], false)
        {
            changed = true;
        } else {
            return Err("压缩请求无法缩小到模型输入限制；原始会话和已有检查点已保留，请减少单条输入或选择更大窗口模型".into());
        }
    }
    Ok(changed)
}

/// Enforce a large decrease when possible, but retain a viable prompt for tiny histories.
pub(super) fn recover(messages: &mut Vec<Value>, budget: u64) -> Result<bool, String> {
    let mut candidate = messages.clone();
    if let Ok(changed) = fit(&mut candidate, budget) {
        if changed {
            *messages = candidate;
            return Ok(true);
        }
    }
    let source_len = messages.len().saturating_sub(1);
    if shrink_largest_content(&mut messages[..source_len], true)
        || remove_oldest_group(messages)
        || shrink_largest_content(&mut messages[..source_len], false)
    {
        return Ok(true);
    }
    Err("压缩输入已无法继续缩小；原始会话和已有检查点已保留".into())
}
