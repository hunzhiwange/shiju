//! 文件链接辅助逻辑，负责从工具输入输出中提取、压缩和生成可展示的文件引用。

use crate::app::agent::tools::ToolRuntimeContext;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// 执行 extract_file_link_blocks 操作，并返回调用方需要的结果。
pub(crate) fn extract_file_link_blocks(s: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut i = 0usize;
    loop {
        let Some(start_rel) = s[i..].find("<file_link>") else {
            break;
        };
        let start = i + start_rel;
        let Some(end_rel) = s[start..].find("</file_link>") else {
            break;
        };
        let end = start + end_rel + "</file_link>".len();
        out.push(s[start..end].to_string());
        i = end;
        if i >= s.len() {
            break;
        }
    }
    out
}

/// 执行 compact_file_link 操作，并返回调用方需要的结果。
pub(crate) fn compact_file_link(s: &str) -> String {
    let start = match s.find("<file_link>") {
        Some(v) => v,
        None => return s.to_string(),
    };
    let end = match s.find("</file_link>") {
        Some(v) => v + "</file_link>".len(),
        None => return s.to_string(),
    };
    let block = &s[start..end];
    let mut path_line: Option<String> = None;
    for line in block.lines() {
        let l = line.trim();
        if let Some(p) = l.strip_prefix("path:") {
            path_line = Some(format!("path: {}", p.trim()));
            break;
        }
    }
    let mut out = String::new();
    out.push_str(&s[..start]);
    if let Some(pl) = path_line {
        out.push_str(&pl);
        out.push('\n');
    }
    out.push_str(&s[end..]);
    out.trim().to_string()
}

fn normalize_file_reference_candidate(input: &str) -> Option<String> {
    let mut value = input.trim();
    if value.is_empty() {
        return None;
    }

    if value.starts_with('[')
        && value.ends_with(')')
        && let Some((_, target)) = value.rsplit_once("](")
    {
        value = target.trim_end_matches(')');
    }

    value = value.trim().trim_matches('`').trim_matches('"').trim_matches('\'').trim();

    if value.is_empty() {
        return None;
    }

    if let Some(stripped) = value.strip_prefix("file:///").or_else(|| value.strip_prefix("file://"))
    {
        value = stripped;
    }

    if let Some((path, _)) = value.split_once("#L") {
        value = path;
    } else if let Some((path, _)) = value.split_once("#line-") {
        value = path;
    }

    let normalized = value.trim();
    if normalized.is_empty() { None } else { Some(normalized.to_string()) }
}

/// 执行 maybe_inject_file_link 操作，并返回调用方需要的结果。
pub(crate) fn maybe_inject_file_link(
    name: &str,
    input: &str,
    ctx: &ToolRuntimeContext,
    output: &str,
) -> String {
    if output.contains("<file_link>") {
        return output.to_string();
    }
    let eligible = matches!(
        name,
        "read"
            | "file_read"
            | "pdf_read"
            | "write"
            | "file_write"
            | "file_edit"
            | "apply_patch"
    );
    if !eligible {
        return output.to_string();
    }
    let Some(p) = extract_file_path_from_input(input) else {
        return output.to_string();
    };
    let Some(link) = build_file_link(ctx, &p) else {
        return output.to_string();
    };
    if output.trim().is_empty() {
        return link;
    }
    format!("{}\n{}", link, output)
}

/// 执行 extract_file_path_from_input 操作，并返回调用方需要的结果。
pub(crate) fn extract_file_path_from_input(input: &str) -> Option<String> {
    let raw = input.trim();
    if raw.is_empty() {
        return None;
    }
    if !raw.starts_with('{') {
        return normalize_file_reference_candidate(raw);
    }
    let v = serde_json::from_str::<Value>(raw).ok()?;
    v.get("filePath")
        .or_else(|| v.get("file_path"))
        .or_else(|| v.get("path"))
        .and_then(|vv| vv.as_str())
        .and_then(normalize_file_reference_candidate)
}

/// 执行 resolve_full_path 操作，并返回调用方需要的结果。
pub(crate) fn resolve_full_path(ctx: &ToolRuntimeContext, p: &str) -> PathBuf {
    let p = p.trim();
    if Path::new(p).is_absolute() {
        return PathBuf::from(p);
    }
    if let Some(root) = ctx.root.as_deref() {
        return PathBuf::from(root).join(p);
    }
    PathBuf::from(p)
}

/// 执行 build_file_link 操作，并返回调用方需要的结果。
pub(crate) fn build_file_link(ctx: &ToolRuntimeContext, p: &str) -> Option<String> {
    let full = resolve_full_path(ctx, p);
    let base = ctx
        .root
        .as_deref()
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let rel = full.strip_prefix(&base).unwrap_or(&full);
    let bytes = full.metadata().map(|m| m.len()).unwrap_or(0);
    let open = format!("file:///{}", full.to_string_lossy());
    Some(format!(
        "<file_link>\npath: {}\nopen: {}\nsize_bytes: {}\n</file_link>",
        rel.to_string_lossy().replace('\\', "/"),
        open,
        bytes
    ))
}
#[cfg(test)]
#[path = "file_links_tests.rs"]
mod file_links_tests;
