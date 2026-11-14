//! Shared structured block constructors, preserving upstream tool identity and references.
use serde_json::Value;
use vw_api_types::external_session::*;
pub use super::tool_normalization::*;
pub use super::block_preview::*;
pub use vw_api_types::external_session::project_content;


/// 参数里第一个存在的非空字符串字段。
pub fn str_field<'a>(input: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| input.get(*key).and_then(Value::as_str))
        .filter(|s| !s.trim().is_empty())
}

/// 参数里第一个存在的非负整数字段（兼容写成字符串的数字）。
fn u64_field(input: &Value, keys: &[&str]) -> Option<u64> {
    keys.iter().find_map(|key| {
        let value = input.get(*key)?;
        value
            .as_u64()
            .or_else(|| value.as_str().and_then(|s| s.trim().parse().ok()))
    })
}

/// OpenAI 形状的 `arguments` 是 JSON 字符串：能解析成对象/数组就解析，否则保持原样。
pub fn parse_arguments(raw: &Value) -> Value {
    match raw {
        Value::String(text) => match serde_json::from_str::<Value>(text) {
            Ok(parsed @ (Value::Object(_) | Value::Array(_))) => parsed,
            _ => raw.clone(),
        },
        other => other.clone(),
    }
}

/// 工具参数预览：字符串原样，其余序列化为紧凑 JSON；空参数为空串。
pub fn input_preview(input: &Value) -> Preview {
    match input {
        Value::Null => preview_chars("", INPUT_PREVIEW_CHARS),
        Value::String(text) => preview_chars(text, INPUT_PREVIEW_CHARS),
        other => preview_chars(&other.to_string(), INPUT_PREVIEW_CHARS),
    }
}

const PATH_KEYS: &[&str] = &[
    "file_path",
    "filePath",
    "path",
    "absolute_path",
    "file",
    "filename",
    "dir_path",
];

/// 按 kind 从参数里提炼 `(title, detail)`（§3.3）。找不到对应字段时 title 为空，
/// 投影退化为 `[Tool: name]`。
pub fn title_from_input(
    kind: ToolKind,
    raw_name: &str,
    server: Option<&str>,
    input: &Value,
) -> (String, Option<String>) {
    let detail_of = |keys: &[&str]| str_field(input, keys).map(one_line_title);
    match kind {
        ToolKind::Shell => (
            str_field(input, &["command", "cmd", "script"])
                .map(title_shell)
                .unwrap_or_default(),
            detail_of(&["description"]),
        ),
        ToolKind::Read => {
            if let Some(path) = str_field(input, PATH_KEYS) {
                title_read(
                    path,
                    u64_field(input, &["offset", "start_line"]),
                    u64_field(input, &["limit"]),
                )
            } else {
                let paths: Vec<&str> = input
                    .get("paths")
                    .and_then(Value::as_array)
                    .map(|items| items.iter().filter_map(Value::as_str).collect())
                    .unwrap_or_default();
                (title_web(&paths), None)
            }
        }
        ToolKind::Search => {
            let path = str_field(input, &["path", "dir_path", "directory", "cwd"]);
            match str_field(input, &["pattern", "query", "regex", "glob", "include"]) {
                Some(pattern) => (title_search(pattern, path), None),
                None => (path.map(title_path).unwrap_or_default(), None),
            }
        }
        ToolKind::Edit | ToolKind::Write => (
            str_field(input, PATH_KEYS)
                .map(title_path)
                .unwrap_or_default(),
            None,
        ),
        ToolKind::Web => {
            let mut items: Vec<&str> = ["urls", "queries"]
                .iter()
                .filter_map(|key| input.get(*key).and_then(Value::as_array))
                .flatten()
                .filter_map(Value::as_str)
                .collect();
            if items.is_empty() {
                items.extend(str_field(input, &["url", "query", "prompt"]));
            }
            (title_web(&items), None)
        }
        ToolKind::Mcp => {
            let server = server.unwrap_or_default();
            let tool = split_mcp_name(raw_name)
                .map(|(_, tool)| tool)
                .or_else(|| {
                    raw_name
                        .strip_prefix(server)
                        .map(|rest| rest.trim_start_matches(['_', '/', '.']))
                })
                .filter(|tool| !tool.is_empty())
                .unwrap_or(raw_name);
            (title_mcp(server, tool), first_string_field(input))
        }
        ToolKind::Agent => (
            str_field(input, &["description", "task_name", "prompt", "message"])
                .map(title_agent)
                .unwrap_or_default(),
            detail_of(&["subagent_type", "agent", "model"]),
        ),
        ToolKind::Ask => {
            let question = input
                .get("questions")
                .and_then(Value::as_array)
                .and_then(|items| items.first())
                .and_then(|first| {
                    first
                        .get("question")
                        .and_then(Value::as_str)
                        .or_else(|| first.as_str())
                })
                .or_else(|| str_field(input, &["question", "prompt"]));
            (question.map(title_ask).unwrap_or_default(), None)
        }
        ToolKind::Todo => {
            let count = input.get("todos").and_then(Value::as_array).map(Vec::len);
            (count.map(title_todo).unwrap_or_default(), None)
        }
        ToolKind::Other => (title_other(raw_name), first_string_field(input)),
    }
}

/// 文本行数（空文本为 0）。
fn text_lines(text: &str) -> u32 {
    saturating_u32(text.lines().count())
}

/// 统计 unified diff（或带行号的 `+12 xxx` 形式）里的增删行数，跳过 `+++`/`---` 文件头。
pub fn count_diff_lines(diff: &str) -> (u32, u32) {
    let (mut added, mut removed) = (0u32, 0u32);
    for line in diff.lines() {
        if line.starts_with("+++") || line.starts_with("---") {
            continue;
        }
        if line.starts_with('+') {
            added = added.saturating_add(1);
        } else if line.starts_with('-') {
            removed = removed.saturating_add(1);
        }
    }
    (added, removed)
}

/// 单文件改动摘要。
pub fn single_file_diff(path: &str, op: DiffOp, added: u32, removed: u32) -> DiffSummary {
    DiffSummary {
        files: vec![DiffFile {
            path: path.to_string(),
            op,
            added,
            removed,
        }],
        added,
        removed,
        full: None,
    }
}

/// 只凭参数估算 Edit/Write 的改动（`old/new` 字符串按行数计、`edits[]` 求和、`content` 视为新增）。
/// 拿到工具自己给的 diff 时应以那个为准。
pub fn diff_from_input(kind: ToolKind, input: &Value) -> Option<DiffSummary> {
    let path = str_field(input, PATH_KEYS)?;
    let pair = |item: &Value| -> Option<(u32, u32)> {
        let old = str_field_allow_empty(item, &["old_string", "oldString", "old_str", "oldText"])?;
        let new = str_field_allow_empty(item, &["new_string", "newString", "new_str", "newText"])?;
        Some((text_lines(new), text_lines(old)))
    };
    match kind {
        ToolKind::Edit => {
            let (added, removed) = if let Some(edits) = input.get("edits").and_then(Value::as_array)
            {
                edits
                    .iter()
                    .filter_map(pair)
                    .fold((0u32, 0u32), |(a, r), (x, y)| {
                        (a.saturating_add(x), r.saturating_add(y))
                    })
            } else {
                pair(input)?
            };
            Some(single_file_diff(path, DiffOp::Update, added, removed))
        }
        ToolKind::Write => {
            let content = str_field_allow_empty(input, &["content", "contents", "text"])?;
            Some(single_file_diff(path, DiffOp::Add, text_lines(content), 0))
        }
        _ => None,
    }
}

fn str_field_allow_empty<'a>(input: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| input.get(*key).and_then(Value::as_str))
}

/// OpenAI 形状的 `tool_calls[]`（`{id, function{name, arguments}}`，也兼容扁平 `{id, name, arguments}`）
/// → ToolCall。`arguments_ref(pointer)` 为参数全文生成引用，`pointer` 是参数在 `tool_calls`
/// 数组内的 JSON Pointer（`/{i}/function/arguments` 或扁平形状的 `/{i}/arguments`）。
pub(super) fn openai_tool_calls(
    calls: Option<&Value>,
    arguments_ref: impl Fn(String) -> Option<ContentRef>,
) -> Vec<SessionBlock> {
    calls
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
        .map(|(i, call)| {
            let name = call
                .pointer("/function/name")
                .or_else(|| call.get("name"))
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let id = call.get("id").and_then(Value::as_str).unwrap_or_default();
            let (pointer, raw) = match call.pointer("/function/arguments") {
                Some(raw) => (format!("/{i}/function/arguments"), Some(raw)),
                None => (format!("/{i}/arguments"), call.get("arguments")),
            };
            let input = raw.map(parse_arguments).unwrap_or(Value::Null);
            tool_call_block(ToolSource::Generic, id, name, &input, || {
                arguments_ref(pointer)
            })
        })
        .collect()
}

/// 由参数构造 `ToolCall`：归一化 kind、提炼标题、参数预览；参数超长时调用 `input_full` 生成引用。
/// Edit/Write 先按参数估算 diff，解析器拿到更准的数据再覆盖。
pub fn tool_call_block(
    source: ToolSource,
    id: impl Into<String>,
    raw_name: &str,
    input: &Value,
    input_full: impl FnOnce() -> Option<ContentRef>,
) -> SessionBlock {
    let NormalizedTool { kind, server } = normalize_tool(source, raw_name, None);
    let (title, detail) = title_from_input(kind, raw_name, server.as_deref(), input);
    let preview = input_preview(input);
    SessionBlock::ToolCall {
        id: id.into(),
        raw_name: raw_name.to_string(),
        kind,
        title,
        detail,
        server,
        input_preview: preview.text,
        input_total_len: preview.total_len,
        input_full: if preview.truncated {
            input_full()
        } else {
            None
        },
        diff: diff_from_input(kind, input),
        by_user: false,
    }
}

/// 构造 `ToolResult`：预览 12 行 / 1200 字，超出时调用 `full` 生成引用。
/// `exit_code` / `duration_ms` / `images` / `saved_path` 由解析器按需补上。
pub fn tool_result_block(
    call_id: impl Into<String>,
    status: ToolStatus,
    text: &str,
    full: impl FnOnce() -> Option<ContentRef>,
) -> SessionBlock {
    let p = preview(text);
    SessionBlock::ToolResult {
        call_id: call_id.into(),
        status,
        preview: p.text,
        total_len: p.total_len,
        line_count: p.line_count,
        truncated: p.truncated,
        full: if p.truncated { full() } else { None },
        exit_code: None,
        duration_ms: None,
        images: Vec::new(),
        saved_path: None,
    }
}

/// 构造 `Thinking`：正文按 [`THINKING_PREVIEW_CHARS`] 截断，超出时调用 `full` 生成引用；
/// 正文为空视为不可见（redacted）。
pub fn thinking_block(
    text: &str,
    summary: Option<String>,
    duration_ms: Option<u64>,
    full: impl FnOnce() -> Option<ContentRef>,
) -> SessionBlock {
    let p = preview_chars(text.trim_end(), THINKING_PREVIEW_CHARS);
    SessionBlock::Thinking {
        redacted: text.trim().is_empty(),
        text: p.text,
        summary: if text.trim().is_empty() { None } else { summary },
        duration_ms,
        full: if p.truncated { full() } else { None },
    }
}

/// 构造 `Text`：超过 [`INLINE_TEXT_MAX_CHARS`] 且能给出引用时只放预览（12 行 / 1200 字）+ `full`。
/// 只用于默认折叠的内容（注入文本）；真人提问与最终回复要完整渲染 Markdown，不走这里。
pub fn large_text_block(
    text: impl Into<String>,
    full: impl FnOnce() -> Option<ContentRef>,
) -> SessionBlock {
    let text = text.into();
    if text.chars().nth(INLINE_TEXT_MAX_CHARS).is_none() {
        return SessionBlock::text(text);
    }
    match full() {
        Some(full) => SessionBlock::Text {
            text: preview(&text).text,
            full: Some(full),
        },
        None => SessionBlock::text(text),
    }
}

/// 构造带长说明的 `Event`（压缩摘要）：超过 [`EVENT_PREVIEW_CHARS`] 且能给出引用时只放预览 + `full`；
/// 拿不到引用时退回带 `…` 的预览，避免把整段摘要塞进 payload。
pub fn summary_event_block(
    kind: EventKind,
    text: &str,
    full: impl FnOnce() -> Option<ContentRef>,
) -> SessionBlock {
    let text = text.trim();
    let p = preview_chars(text, EVENT_PREVIEW_CHARS);
    if !p.truncated {
        return SessionBlock::event(kind, Some(p.text), None);
    }
    match full() {
        Some(full) => SessionBlock::Event {
            kind,
            text: Some(p.text),
            url: None,
            full: Some(full),
        },
        None => SessionBlock::event(kind, Some(format!("{}…", p.text)), None),
    }
}


/// 由替换前后的文本估算 `(+added, -removed)` 行数：去掉首尾相同的行后，
/// 剩余的旧行算删除、新行算新增。没有结构化 patch 时（如 Claude Edit 的参数）用。
pub fn line_change_counts(old: &str, new: &str) -> (u32, u32) {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();
    let prefix = old_lines
        .iter()
        .zip(&new_lines)
        .take_while(|(a, b)| a == b)
        .count();
    let suffix = old_lines[prefix..]
        .iter()
        .rev()
        .zip(new_lines[prefix..].iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    (
        saturating_u32(new_lines.len() - prefix - suffix),
        saturating_u32(old_lines.len() - prefix - suffix),
    )
}
