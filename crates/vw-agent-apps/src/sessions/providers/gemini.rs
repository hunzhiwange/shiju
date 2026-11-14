use std::path::Path;

use serde_json::Value;

use vw_api_types::external_session::*;

use crate::sessions::blocks::{
    count_diff_lines, single_file_diff, thinking_block, tool_call_block,
    tool_result_block, ToolSource,
};
use crate::sessions::utils::truncate_summary;
use super::opencode::parse_timestamp_to_ms;

#[path = "gemini_document.rs"]
mod document;
pub(crate) use document::parse_session_document;

/// Scan only the selected execution-host Gemini instance root.
pub fn scan_sessions(config_dir: &Path, identity: &ExternalSessionIdentity) -> Result<Vec<SessionMeta>, String> {
    if !config_dir.is_absolute() || identity.provider != SessionProvider::Gemini {
        return Err("invalid Gemini instance root or provider".into());
    }
    let tmp = config_dir.join("tmp");
    if !tmp.exists() { return Ok(Vec::new()); }
    ensure_directory(&tmp)?;
    let mut sessions = Vec::new();
    for project in std::fs::read_dir(&tmp).map_err(|e| format!("Gemini projects: {e}"))? {
        let project = project.map_err(|e| format!("Gemini project: {e}"))?;
        if !project.file_type().map_err(|e| format!("Gemini project type: {e}"))?.is_dir() { continue; }
        let chats = project.path().join("chats");
        if !chats.exists() { continue; }
        ensure_directory(&chats)?;
        let root_file = project.path().join(".project_root");
        let project_dir = match std::fs::symlink_metadata(&root_file) {
            Ok(meta) if meta.is_file() && !meta.file_type().is_symlink() => std::fs::read_to_string(&root_file).ok(),
            _ => None,
        };
        for file in std::fs::read_dir(&chats).map_err(|e| format!("Gemini chats: {e}"))? {
            let file = file.map_err(|e| format!("Gemini chat: {e}"))?;
            if !file.file_type().map_err(|e| format!("Gemini chat type: {e}"))?.is_file() { continue; }
            let path = file.path();
            if !is_session_file(&path) { continue; }
            let value = read_session_document(&path)?;
            let session_id = value.get("sessionId").and_then(Value::as_str).filter(|id| !id.is_empty())
                .ok_or_else(|| "Gemini session has no sessionId".to_string())?;
            let created_at = value.get("startTime").and_then(parse_timestamp_to_ms);
            let title = value.get("messages").and_then(Value::as_array).and_then(|messages| messages.iter()
                .filter(|m| m.get("type").and_then(Value::as_str) == Some("user"))
                .map(|m| content_text(m.get("content"))).find(|text| !is_ignored_user_text(text)))
                .map(|text| truncate_summary(&text, 160));
            let mut bound = identity.clone();
            bound.session_id = ExternalSessionId(session_id.to_string());
            bound.source = SessionSource::File { relative_path: path.strip_prefix(config_dir)
                .map_err(|_| "Gemini path outside instance")?.to_string_lossy().into_owned() };
            sessions.push(SessionMeta {
                identity: bound, title: title.clone(), summary: title,
                project_dir: project_dir.clone(), created_at,
                last_active_at: value.get("lastUpdated").and_then(parse_timestamp_to_ms).or(created_at),
                source_path: Some(path.to_string_lossy().into_owned()),
                capabilities: SessionCapabilities { read: true, resume: true, delete: true, export_markdown: true },
            });
        }
    }
    sessions.sort_by(|a,b| b.last_active_at.cmp(&a.last_active_at).then(a.identity.session_id.0.cmp(&b.identity.session_id.0)));
    Ok(sessions)
}

fn ensure_directory(path: &Path) -> Result<(), String> {
    let meta = std::fs::symlink_metadata(path).map_err(|e| format!("Gemini directory: {e}"))?;
    if !meta.is_dir() || meta.file_type().is_symlink() { return Err("invalid Gemini directory".into()); }
    Ok(())
}

/// 会话文件：旧版 `.json`，或新版 Gemini CLI 写的 `.jsonl`。旧文件被 resume 迁移后
/// 会留下同名 `.json`，此时只认 `.jsonl`，避免同一会话列两次。
pub(crate) fn is_session_file(path: &Path) -> bool {
    match path.extension().and_then(|e| e.to_str()) {
        Some("jsonl") => true,
        Some("json") => !path.with_extension("jsonl").exists(),
        _ => false,
    }
}

/// Gemini CLI 注入或非提问的用户文本（同上游 `isIgnoredUserContent`）
fn is_ignored_user_text(text: &str) -> bool {
    let t = text.trim();
    t.is_empty() || t.starts_with('/') || t.starts_with('?') || is_injected_user_text(t)
}

/// CLI 注入的上下文（环境信息、hook 输出），不是用户的提问
fn is_injected_user_text(text: &str) -> bool {
    let t = text.trim_start();
    t.starts_with("<session_context>") || t.starts_with("<hook_context>")
}

fn read_session_document(path: &Path) -> Result<Value, String> {
    let data = std::fs::read_to_string(path).map_err(|e| format!("Gemini session read failed: {e}"))?;
    parse_session_document(&data).ok_or_else(|| "Gemini session JSON is malformed".to_string())
}

/// Relative paths are resolved only under this selected instance's tmp directory.
pub fn load_messages(config_dir: &Path, relative_path: &Path) -> Result<Vec<SessionMessage>, String> {
    let path = checked_path(config_dir, relative_path)?;
    load_file_messages(&path)
}

fn checked_path(config_dir: &Path, relative: &Path) -> Result<std::path::PathBuf, String> {
    use std::path::Component;
    if !config_dir.is_absolute() || relative.is_absolute()
        || relative.components().any(|c| !matches!(c, Component::Normal(_))) {
        return Err("invalid Gemini relative path".into());
    }
    let parts: Vec<_> = relative.components().collect();
    if parts.len() != 4 || parts[0].as_os_str() != "tmp" || parts[2].as_os_str() != "chats" {
        return Err("Gemini session must be under tmp/<project>/chats".into());
    }
    let mut path = config_dir.to_path_buf();
    for part in parts {
        path.push(part.as_os_str());
        let meta = std::fs::symlink_metadata(&path).map_err(|e| format!("Gemini source: {e}"))?;
        if meta.file_type().is_symlink() { return Err("Gemini symbolic links are not supported".into()); }
    }
    if !path.is_file() || !matches!(path.extension().and_then(|v| v.to_str()), Some("json" | "jsonl")) {
        return Err("invalid Gemini session file".into());
    }
    Ok(path)
}

fn load_file_messages(path: &Path) -> Result<Vec<SessionMessage>, String> {
    let value = read_session_document(path)?;

    let messages = value
        .get("messages")
        .and_then(Value::as_array)
        .ok_or_else(|| "No messages array found".to_string())?;

    // 全文引用指向会话文件本身：rel_path 相对 provider 根（`~/.gemini/tmp`），即 `<hash>/chats/<file>`
    let rel_path = relative_to_tmp_root(path);
    let file_ref = |pointer: String| -> Option<ContentRef> {
        Some(ContentRef::File {
            rel_path: rel_path.clone(),
            pointer,
        })
    };

    let mut result = Vec::new();
    for (index, msg) in messages.iter().enumerate() {
        let ts = msg.get("timestamp").and_then(parse_timestamp_to_ms);
        let base = format!("/messages/{index}");
        let mut message = match msg.get("type").and_then(Value::as_str) {
            Some("user") => {
                let text = content_text(msg.get("content"));
                if text.trim().is_empty() {
                    continue;
                }
                let injected = is_injected_user_text(&text);
                let mut message =
                    SessionMessage::from_blocks("user", ts, vec![SessionBlock::text(text)]);
                message.injected = injected;
                message
            }
            Some("gemini") => {
                let blocks = gemini_blocks(msg, &base, &file_ref);
                let mut message = SessionMessage::from_blocks("assistant", ts, blocks);
                message.meta = gemini_meta(msg);
                message
            }
            Some(kind @ ("info" | "error")) => {
                let text = content_text(msg.get("content"));
                if text.trim().is_empty() {
                    continue;
                }
                let event_kind = if kind == "info" {
                    EventKind::Info
                } else {
                    EventKind::Error
                };
                let mut message = SessionMessage::from_blocks(
                    "system",
                    ts,
                    vec![SessionBlock::event(event_kind, Some(text), None)],
                );
                // info（登录、刷新等提示）默认折叠
                message.injected = kind == "info";
                message
            }
            Some(_) | None => continue,
        };
        if message.is_empty() {
            continue;
        }
        message.id = msg.get("id").and_then(Value::as_str).map(str::to_string);
        result.push(message);
    }

    assign_turn_ids(&mut result);
    Ok(result)
}

/// 多条 `thoughts[{subject, description}]` 合并后的思考：`summary` 为各 subject 以 ` · ` 连接，
/// `text` 为 `**subject**\n\ndescription` 以空行连接。
pub(crate) struct MergedThoughts {
    pub summary: String,
    pub text: String,
}

/// 合并 Gemini 的 `thoughts` 数组；不是对象数组、或没有任何非空 subject/description 时返回 `None`。
/// 解析器生成预览与按引用取全文共用这一份格式。
pub(crate) fn format_thoughts(value: &Value) -> Option<MergedThoughts> {
    let items = value.as_array()?;
    let mut thoughts = Vec::new();
    for item in items {
        let object = item.as_object()?;
        let field = |key: &str| object.get(key).and_then(Value::as_str).unwrap_or("").trim();
        let (subject, description) = (field("subject"), field("description"));
        if !subject.is_empty() || !description.is_empty() {
            thoughts.push((subject, description));
        }
    }
    if thoughts.is_empty() {
        return None;
    }
    let summary = thoughts
        .iter()
        .map(|(subject, _)| *subject)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" · ");
    let text = thoughts
        .iter()
        .map(
            |(subject, description)| match (subject.is_empty(), description.is_empty()) {
                (false, false) => format!("**{subject}**\n\n{description}"),
                (false, true) => format!("**{subject}**"),
                _ => description.to_string(),
            },
        )
        .collect::<Vec<_>>()
        .join("\n\n");
    Some(MergedThoughts { summary, text })
}

/// 会话文件相对 `tmp/` 的路径：取末尾三段 `<hash>/chats/<file>`（不足三段时取已有部分）。
fn relative_to_tmp_root(path: &Path) -> String {
    let parts: Vec<String> = path
        .components()
        .rev()
        .take(3)
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();
    parts.into_iter().rev().collect::<Vec<_>>().join("/")
}

/// Gemini content 可能是字符串，也可能是 `[{text}]` 数组。
fn content_text(content: Option<&Value>) -> String {
    match content {
        Some(Value::String(s)) => s.to_string(),
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|item| item.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

/// `type=gemini` 消息：思考（多条合并为一块）→ 正文 → 工具调用与结果（按生成顺序）。
fn gemini_blocks(
    msg: &Value,
    base: &str,
    file_ref: &dyn Fn(String) -> Option<ContentRef>,
) -> Vec<SessionBlock> {
    let mut blocks = Vec::new();

    if let Some(thoughts) = msg.get("thoughts").and_then(format_thoughts) {
        // 合并后的正文与取回的全文同一口径：引用指向整个数组，由 `content::resolve_content_ref`
        // 按 `format_thoughts` 格式化
        blocks.push(thinking_block(
            &thoughts.text,
            (!thoughts.summary.is_empty()).then_some(thoughts.summary),
            None,
            || file_ref(format!("{base}/thoughts")),
        ));
    }

    let text = content_text(msg.get("content"));
    if !text.trim().is_empty() {
        blocks.push(SessionBlock::text(text));
    }

    for (j, call) in msg
        .get("toolCalls")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
    {
        let Some(name) = call.get("name").and_then(Value::as_str) else {
            continue;
        };
        let call_base = format!("{base}/toolCalls/{j}");
        let id = call
            .get("id")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| format!("{name}-{j}"));
        let args = call.get("args").cloned().unwrap_or(Value::Null);
        let mut call_block = tool_call_block(ToolSource::Gemini, id.clone(), name, &args, || {
            file_ref(format!("{call_base}/args"))
        });

        let (output, pointer) = tool_output(call, &call_base);
        // resultDisplay 是 FileDiff 时用真实 diff 覆盖参数估算
        if let Some(diff_text) = call
            .pointer("/resultDisplay/fileDiff")
            .and_then(Value::as_str)
        {
            if let SessionBlock::ToolCall { diff, title, .. } = &mut call_block {
                let (added, removed) = count_diff_lines(diff_text);
                let path = call
                    .pointer("/resultDisplay/filePath")
                    .or_else(|| call.pointer("/resultDisplay/fileName"))
                    .and_then(Value::as_str)
                    .unwrap_or(title.as_str())
                    .to_string();
                let op = diff
                    .as_ref()
                    .and_then(|d| d.files.first())
                    .map_or(DiffOp::Update, |f| f.op);
                let mut summary = single_file_diff(&path, op, added, removed);
                summary.full = file_ref(format!("{call_base}/resultDisplay/fileDiff"));
                *diff = Some(summary);
            }
        }
        blocks.push(call_block);

        let status = match call.get("status").and_then(Value::as_str) {
            Some("success" | "completed") => ToolStatus::Success,
            Some("error") => ToolStatus::Error,
            Some("cancelled" | "canceled") => ToolStatus::Interrupted,
            Some("executing" | "scheduled" | "validating" | "awaiting_approval") => {
                ToolStatus::Pending
            }
            _ => ToolStatus::Unknown,
        };
        blocks.push(tool_result_block(id, status, &output, || {
            pointer.and_then(file_ref)
        }));
    }

    blocks
}

/// 工具输出文本与其 JSON Pointer（能精确指到字符串时才给）。
/// 优先 `resultDisplay`（字符串或 FileDiff），否则 `result[].functionResponse.response.output|error`。
fn tool_output(call: &Value, call_base: &str) -> (String, Option<String>) {
    match call.get("resultDisplay") {
        Some(Value::String(text)) if !text.is_empty() => {
            return (text.clone(), Some(format!("{call_base}/resultDisplay")));
        }
        Some(display @ Value::Object(_)) => {
            if let Some(diff) = display.get("fileDiff").and_then(Value::as_str) {
                return (
                    diff.to_string(),
                    Some(format!("{call_base}/resultDisplay/fileDiff")),
                );
            }
            return (display.to_string(), None);
        }
        _ => {}
    }
    for (k, item) in call
        .get("result")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
    {
        for key in ["output", "error"] {
            if let Some(text) = item
                .pointer(&format!("/functionResponse/response/{key}"))
                .and_then(Value::as_str)
            {
                return (
                    text.to_string(),
                    Some(format!(
                        "{call_base}/result/{k}/functionResponse/response/{key}"
                    )),
                );
            }
        }
    }
    match call.get("result") {
        Some(Value::String(text)) => (text.clone(), Some(format!("{call_base}/result"))),
        Some(Value::Null) | None => (String::new(), None),
        Some(other) => (other.to_string(), None),
    }
}

/// `tokens{input, output, cached, thoughts, tool, total}` + `model` → meta（0 视为缺省）。
fn gemini_meta(msg: &Value) -> Option<MessageMeta> {
    let tokens = msg.get("tokens");
    let count = |key: &str| {
        tokens
            .and_then(|t| t.get(key))
            .and_then(Value::as_u64)
            .filter(|n| *n > 0)
    };
    let meta = MessageMeta {
        model: msg
            .get("model")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(str::to_string),
        input_tokens: count("input"),
        output_tokens: count("output"),
        cache_read_tokens: count("cached"),
        reasoning_tokens: count("thoughts"),
        ..MessageMeta::default()
    };
    (meta != MessageMeta::default()).then_some(meta)
}


fn assign_turn_ids(messages: &mut [SessionMessage]) {
    let mut turn = 0usize;
    for message in messages {
        if message.role == "user" && !message.injected { turn += 1; }
        message.turn_id = Some(format!("t{turn}"));
    }
}
