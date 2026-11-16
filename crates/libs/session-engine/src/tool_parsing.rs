//! 文本工具调用解析逻辑，负责识别用户消息中的斜杠工具调用并校验工具名。

use serde_json::Value;
use std::collections::HashSet;

/// 执行 parse_tool_at 操作，并返回调用方需要的结果。
pub fn parse_tool_at(
    lines: &[&str],
    start: usize,
    allowed_tools: &HashSet<String>,
) -> Option<(String, String, usize)> {
    let first = lines.get(start)?.trim();
    let text = first.strip_prefix('/')?;
    let mut parts = text.splitn(2, ' ');
    let name = parts.next()?.trim();
    if name.is_empty() {
        return None;
    }
    if !is_valid_tool_name(name) || !allowed_tools.contains(name) {
        return None;
    }
    let rest = parts.next().unwrap_or("").trim();
    if !rest.is_empty() {
        if rest.trim_start().starts_with('{') && serde_json::from_str::<Value>(rest).is_err() {
            return None;
        }
        return Some((name.to_string(), rest.to_string(), 1));
    }

    let mut j = start + 1;
    while j < lines.len() && lines[j].trim().is_empty() {
        j += 1;
    }
    if j >= lines.len() {
        return Some((name.to_string(), String::new(), 1));
    }

    if matches!(name, "read" | "file_read") {
        let mut k = j;
        while k < lines.len() && k - j < 32 {
            let t = lines[k].trim();
            if t.starts_with('/') {
                break;
            }
            if let Some(path) = t.strip_prefix('@') {
                let path = path.trim();
                if !path.is_empty() {
                    return Some((name.to_string(), path.to_string(), k - start + 1));
                }
            }
            k += 1;
        }
    }

    if !lines[j].trim_start().starts_with('{') {
        return Some((name.to_string(), String::new(), 1));
    }

    let mut buf = String::new();
    let mut last_ok: Option<usize> = None;
    let mut k = j;
    while k < lines.len() && k - j < 256 {
        if !buf.is_empty() {
            buf.push('\n');
        }
        buf.push_str(lines[k].trim_end());
        if serde_json::from_str::<Value>(buf.trim()).is_ok() {
            last_ok = Some(k);
        }
        if let Some(ok) = last_ok
            && ok == k
        {
            return Some((name.to_string(), buf.trim().to_string(), ok - start + 1));
        }
        k += 1;
    }

    if last_ok.is_none() {
        return None;
    }
    Some((name.to_string(), buf.trim().to_string(), k - start))
}

/// 执行 is_valid_tool_name 操作，并返回调用方需要的结果。
pub fn is_valid_tool_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.is_empty() || bytes.len() > 32 {
        return false;
    }
    bytes.iter().all(|b| matches!(b, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_'))
}

/// 执行 query_has_any_tool_calls_with_allowed 操作，并返回调用方需要的结果。
pub fn query_has_any_tool_calls_with_allowed(
    query: &str,
    allowed_tools: &HashSet<String>,
) -> bool {
    let lines: Vec<&str> = query.lines().collect();
    lines.iter().enumerate().any(|(i, _)| parse_tool_at(&lines, i, allowed_tools).is_some())
}

/// 执行 query_has_any_tool_calls 操作，并返回调用方需要的结果。
pub fn query_has_any_tool_calls<H: crate::helpers::HelperHost>(query: &str) -> bool {
    let allowed = crate::helpers::allowed_tool_ids::<H>(None);
    query_has_any_tool_calls_with_allowed(query, &allowed)
}
#[cfg(test)]
#[path = "tool_parsing_tests.rs"]
mod tool_parsing_tests;
