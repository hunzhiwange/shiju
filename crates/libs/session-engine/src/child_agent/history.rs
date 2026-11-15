//! Visible child history, excluding tool traces and private reasoning.
use serde_json::{Value, json};
pub use session_port::child_agent::ForkTurns;

fn is_think_tag_remainder(remainder: &str) -> bool {
    if remainder.is_empty() || remainder.chars().next().is_some_and(char::is_whitespace) {
        return true;
    }
    remainder.strip_prefix("ing").is_some_and(|after| {
        after.is_empty() || after.chars().next().is_some_and(char::is_whitespace)
    })
}

fn find_think_open_tag(s: &str) -> Option<(usize, usize)> {
    let mut search_from = 0usize;
    while let Some(pos_rel) = s[search_from..].find("<think") {
        let pos = search_from + pos_rel;
        if s[pos..].starts_with("</think") {
            search_from = pos + 1;
            continue;
        }
        let Some(tag_end_rel) = s[pos..].find('>') else {
            return None;
        };
        let tag = &s[pos + "<think".len()..pos + tag_end_rel];
        if is_think_tag_remainder(tag) {
            return Some((pos, tag_end_rel + 1));
        }
        search_from = pos + 1;
    }
    None
}

fn find_think_close_tag(s: &str) -> Option<(usize, usize)> {
    let mut search_from = 0usize;
    while let Some(pos_rel) = s[search_from..].find("</think") {
        let pos = search_from + pos_rel;
        let Some(tag_end_rel) = s[pos..].find('>') else {
            return None;
        };
        let tag = &s[pos + "</think".len()..pos + tag_end_rel];
        if is_think_tag_remainder(tag) {
            return Some((pos, tag_end_rel + 1));
        }
        search_from = pos + 1;
    }
    None
}

fn strip_think_blocks(output: &str) -> String {
    let mut visible = String::with_capacity(output.len());
    let mut cursor = 0;
    while let Some((rel_start, open_len)) = find_think_open_tag(&output[cursor..]) {
        let start = cursor + rel_start;
        let open_end = start + open_len;
        let Some((rel_close, close_len)) = find_think_close_tag(&output[open_end..]) else {
            visible.push_str(&output[cursor..start]);
            cursor = output.len();
            break;
        };
        visible.push_str(&output[cursor..start]);
        cursor = open_end + rel_close + close_len;
    }
    visible.push_str(&output[cursor..]);
    visible
}

fn tool_block_end(output: &str, start: usize) -> Option<usize> {
    if start != 0 && !output[..start].ends_with('\n') {
        return None;
    }
    let tail = output.get(start..)?;
    let line_end = tail.find('\n')?;
    let tool_name = tail[..line_end].strip_prefix("tool ")?.trim();
    if tool_name.is_empty() {
        return None;
    }
    let json_start = start + line_end + 1;
    let mut values =
        serde_json::Deserializer::from_str(output.get(json_start..)?).into_iter::<Value>();
    values.next()?.ok()?;
    Some(json_start + values.byte_offset())
}

pub fn visible_answer_from_execution_output(output: &str) -> String {
    let visible = strip_think_blocks(output);
    let mut cursor = 0;
    let mut search_from = 0;
    let mut saw_tool_block = false;
    while let Some(relative_start) = visible[search_from..].find("tool ") {
        let start = search_from + relative_start;
        let Some(end) = tool_block_end(&visible, start) else {
            search_from = start + "tool ".len();
            continue;
        };
        saw_tool_block = true;
        cursor = end;
        search_from = end;
    }
    let final_segment = visible[cursor..].trim();
    if saw_tool_block { final_segment.to_string() } else { visible.trim().to_string() }
}

pub fn snapshot(messages: &[Value]) -> Vec<Value> {
    messages
        .iter()
        .filter_map(|message| {
            let role = message.get("role")?.as_str()?;
            let text = message.get("content")?.as_str()?;
            let content = match role {
                "user" => text.to_string(),
                "assistant" => visible_answer_from_execution_output(text),
                _ => return None,
            };
            if content.trim().is_empty() {
                return None;
            }
            Some(json!({"role":role,"content":content}))
        })
        .collect()
}

#[cfg(test)]
#[path = "history_tests.rs"]
mod tests;
