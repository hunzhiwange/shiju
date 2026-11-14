use serde_json::Value;
use super::tool_normalization::{PREVIEW_LINES, PREVIEW_CHARS, TITLE_CHARS};

/// 截断后的预览与全文统计（长度均按字符计）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preview {
    pub text: String,
    /// 全文字符数
    pub total_len: u32,
    /// 全文行数（空文本为 0；末尾换行不额外计一行）
    pub line_count: u32,
    /// 预览是否短于全文（忽略末尾换行）
    pub truncated: bool,
}

/// 工具结果预览：前 [`PREVIEW_LINES`] 行且不超过 [`PREVIEW_CHARS`] 字符。
pub fn preview(text: &str) -> Preview {
    preview_with(text, PREVIEW_LINES, PREVIEW_CHARS)
}

/// 只按字符截断（参数、思考正文）。
pub fn preview_chars(text: &str, max_chars: usize) -> Preview {
    preview_with(text, usize::MAX, max_chars)
}

pub fn preview_with(text: &str, max_lines: usize, max_chars: usize) -> Preview {
    let total_len = saturating_u32(text.chars().count());
    let line_count = saturating_u32(text.lines().count());

    let mut end = text.len();
    if max_lines > 0 {
        if let Some((idx, _)) = text.match_indices('\n').nth(max_lines - 1) {
            end = idx;
        }
    } else {
        end = 0;
    }
    if let Some((idx, _)) = text[..end].char_indices().nth(max_chars) {
        end = idx;
    }

    let truncated = !text[end..].trim_end_matches(['\r', '\n']).is_empty();
    let kept = text[..end].trim_end_matches(['\r', '\n']);
    Preview {
        text: kept.to_string(),
        total_len,
        line_count,
        truncated,
    }
}

pub fn saturating_u32(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// base64 长度 → 解码后字节数估算（`ImageRef.size`）
pub fn estimate_base64_size(base64_len: usize) -> u32 {
    saturating_u32(base64_len / 4 * 3)
}

// ─── 标题提炼（§3.3）：输出 ≤ TITLE_CHARS，多行取首行并加 `…` ─────────────────

/// 截成单行标题：取首个非空行，超过 [`TITLE_CHARS`] 或有后续行时加 `…`。
pub fn one_line_title(text: &str) -> String {
    let trimmed = text.trim();
    let mut lines = trimmed.lines();
    let first = lines.next().unwrap_or_default().trim_end();
    let more_lines = lines.any(|line| !line.trim().is_empty());
    let mut title: String = first.chars().take(TITLE_CHARS).collect();
    if more_lines || first.chars().count() > TITLE_CHARS {
        title.push('…');
    }
    title
}

/// shell：命令文本首行。Codex 的 shell 包装去除、exec JS 中取 cmd 等由 P1b 补充。
pub fn title_shell(command: &str) -> String {
    one_line_title(command)
}

/// read：标题为路径；`detail` 为行范围 `:start-end`（offset 从 1 起算，limit 为行数）。
pub fn title_read(path: &str, offset: Option<u64>, limit: Option<u64>) -> (String, Option<String>) {
    let detail = match (offset, limit) {
        (Some(start), Some(limit)) if limit > 0 => Some(format!(":{start}-{}", start.saturating_add(limit - 1))),
        (Some(start), None) => Some(format!(":{start}")),
        (None, Some(limit)) if limit > 0 => Some(format!(":1-{limit}")),
        _ => None,
    };
    (one_line_title(path), detail)
}

/// search：`pattern in path`（没有 path 时只给 pattern）。
pub fn title_search(pattern: &str, path: Option<&str>) -> String {
    match path.filter(|p| !p.is_empty()) {
        Some(path) => one_line_title(&format!("{pattern} in {path}")),
        None => one_line_title(pattern),
    }
}

/// edit / write：路径。
pub fn title_path(path: &str) -> String {
    one_line_title(path)
}

/// web：URL 或 query；多个用 `, ` 连接。
pub fn title_web(items: &[&str]) -> String {
    one_line_title(&items.join(", "))
}

/// mcp：`server.tool`。
pub fn title_mcp(server: &str, tool: &str) -> String {
    one_line_title(&format!("{server}.{tool}"))
}

/// agent：description / task_name。
pub fn title_agent(description: &str) -> String {
    one_line_title(description)
}

/// ask：首个问题文本。
pub fn title_ask(question: &str) -> String {
    one_line_title(question)
}

/// todo：条目数。
pub fn title_todo(count: usize) -> String {
    format!("{count} 项")
}

/// other：原名。
pub fn title_other(raw_name: &str) -> String {
    one_line_title(raw_name)
}

/// 参数里第一个非空字符串字段（mcp / other 的 `detail`）。
pub fn first_string_field(input: &Value) -> Option<String> {
    input
        .as_object()?
        .values()
        .find_map(|v| v.as_str().filter(|s| !s.trim().is_empty()))
        .map(one_line_title)
}

