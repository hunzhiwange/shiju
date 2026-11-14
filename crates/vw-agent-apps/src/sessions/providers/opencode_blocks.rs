//! OpenCode 的 part → block 映射（§4.4）。文件存储、SQLite v1、SQLite v2 三条读取路径
//! 都把一条消息的 parts 交给 [`message_from_parts`]，只在「全文引用指向哪里」上不同
//! （见 [`PartLocator`]）。

use serde_json::Value;

use vw_api_types::external_session::{
    ContentRef, DiffOp, EventKind, ImageRef, ImageSource, MessageMeta, SessionBlock,
    SessionMessage, StepPhase, ToolStatus,
};

use crate::sessions::blocks::{
    count_diff_lines, estimate_base64_size, single_file_diff, str_field, thinking_block,
    title_path, tool_call_block, tool_result_block, ToolSource,
};
use super::opencode::parse_timestamp_to_ms;

/// 一个 part 在源存储里的位置，用来生成按需取全文的 [`ContentRef`]。
#[derive(Debug, Clone)]
pub(super) enum PartLocator {
    /// SQLite 行：`table` 白名单内（`part` / `session_message`），`base` 是 part 在列 JSON 里的前缀
    Sqlite {
        table: &'static str,
        id: String,
        base: String,
    },
    /// 文件存储：相对 storage 根的 part 文件路径（`part/<msgId>/<partId>.json`）
    File { rel_path: String },
}

impl PartLocator {
    fn content_ref(&self, sub_pointer: &str) -> Option<ContentRef> {
        Some(match self {
            PartLocator::Sqlite { table, id, base } => ContentRef::Sqlite {
                table: (*table).to_string(),
                id: id.clone(),
                column: "data".to_string(),
                pointer: format!("{base}{sub_pointer}"),
            },
            PartLocator::File { rel_path } => ContentRef::File {
                rel_path: rel_path.clone(),
                pointer: sub_pointer.to_string(),
            },
        })
    }
}

/// 把一条 OpenCode 消息（`message.data` + 有序 parts）转成 [`SessionMessage`]。
///
/// `info` 是消息本体（v1 `message.data` / 文件存储的 message JSON / v2 `session_message.data`），
/// 用来取 meta 与消息级错误；`role` 由调用方给出（v2 的角色在 `type` 列）。
pub(super) fn message_from_parts(
    role: &str,
    id: Option<String>,
    ts: Option<i64>,
    info: &Value,
    parts: &[(PartLocator, Value)],
) -> SessionMessage {
    let mut blocks = Vec::new();
    for (locator, part) in parts {
        push_part_blocks(&mut blocks, locator, part);
    }
    if let Some(event) = message_error_event(info) {
        blocks.push(event);
    }

    let mut message = SessionMessage::from_blocks(role, ts, blocks);
    message.id = id;
    if role == "assistant" {
        message.meta = message_meta(info);
    }
    message
}

fn push_part_blocks(blocks: &mut Vec<SessionBlock>, locator: &PartLocator, part: &Value) {
    match part.get("type").and_then(Value::as_str) {
        Some("text") => {
            // synthetic（OpenCode 自己拼进去的提示）与 ignored 的文本不属于对话
            if part.get("synthetic").and_then(Value::as_bool) == Some(true)
                || part.get("ignored").and_then(Value::as_bool) == Some(true)
            {
                return;
            }
            if let Some(text) = part
                .get("text")
                .and_then(Value::as_str)
                .filter(|t| !t.trim().is_empty())
            {
                blocks.push(SessionBlock::text(text.to_string()));
            }
        }
        Some("reasoning") => {
            let text = part.get("text").and_then(Value::as_str).unwrap_or("");
            blocks.push(thinking_block(text, None, part_duration(part), || {
                locator.content_ref("/text")
            }));
        }
        Some("step-start") => blocks.push(SessionBlock::Step {
            phase: StepPhase::Start,
            tokens: None,
            cost_usd: None,
            reason: None,
        }),
        Some("step-finish") => blocks.push(SessionBlock::Step {
            phase: StepPhase::Finish,
            tokens: part.get("tokens").and_then(total_tokens),
            cost_usd: part.get("cost").and_then(Value::as_f64),
            reason: part
                .get("reason")
                .and_then(Value::as_str)
                .map(str::to_string),
        }),
        Some("tool") => push_tool_blocks(blocks, locator, part),
        Some("agent") => {
            if let Some(name) = part.get("name").and_then(Value::as_str) {
                blocks.push(SessionBlock::event(
                    EventKind::Other,
                    Some(format!("@{name}")),
                    None,
                ));
            }
        }
        Some("patch") => push_patch_blocks(blocks, part),
        Some("file") => push_file_block(blocks, locator, part),
        // snapshot / compaction / subtask / retry 等不进入阅读视图
        _ => {}
    }
}

/// `tool{callID, tool, state}` → ToolCall + ToolResult（v2 的工具项字段为 `name`/`id`）。
fn push_tool_blocks(blocks: &mut Vec<SessionBlock>, locator: &PartLocator, part: &Value) {
    let name = part
        .get("tool")
        .or_else(|| part.get("name"))
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let call_id = part
        .get("callID")
        .or_else(|| part.get("id"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let state = part.get("state");
    let input = state
        .and_then(|s| s.get("input"))
        .or_else(|| part.get("input"))
        .cloned()
        .unwrap_or(Value::Null);
    let metadata = state.and_then(|s| s.get("metadata"));
    let state_title = state
        .and_then(|s| s.get("title"))
        .and_then(Value::as_str)
        .filter(|t| !t.trim().is_empty());

    let mut call = tool_call_block(ToolSource::OpenCode, call_id.clone(), name, &input, || {
        locator.content_ref("/state/input")
    });
    if let SessionBlock::ToolCall {
        kind, title, diff, ..
    } = &mut call
    {
        use vw_api_types::external_session::ToolKind;
        // edit/write 的 state.title 是相对项目的路径，比参数里的绝对路径更短
        if matches!(kind, ToolKind::Edit | ToolKind::Write) {
            if let Some(t) = state_title {
                *title = title_path(t);
            }
        }
        if title.is_empty() {
            if let Some(t) = state_title {
                *title = title_path(t);
            }
        }
        // 工具自己给的改动统计优先于参数估算（字段名待核实：filediff / diff / exists）
        if let Some(meta) = metadata {
            let path = str_field(&input, &["filePath", "file_path", "path"])
                .unwrap_or(title.as_str())
                .to_string();
            let op = if *kind == ToolKind::Write {
                if meta.get("exists").and_then(Value::as_bool) == Some(true) {
                    DiffOp::Update
                } else {
                    DiffOp::Add
                }
            } else {
                DiffOp::Update
            };
            let counts = match (
                meta.pointer("/filediff/additions").and_then(Value::as_u64),
                meta.pointer("/filediff/deletions").and_then(Value::as_u64),
            ) {
                (Some(a), Some(d)) => Some((saturate(a), saturate(d))),
                _ => meta
                    .get("diff")
                    .and_then(Value::as_str)
                    .map(count_diff_lines),
            };
            if let Some((added, removed)) = counts {
                let mut summary = single_file_diff(&path, op, added, removed);
                if meta.get("diff").and_then(Value::as_str).is_some() {
                    summary.full = locator.content_ref("/state/metadata/diff");
                }
                *diff = Some(summary);
            } else if let Some(d) = diff.as_mut() {
                for file in &mut d.files {
                    file.op = op;
                }
            }
        }
    }
    blocks.push(call);

    let status_raw = state.and_then(|s| s.get("status")).and_then(Value::as_str);
    let error = state
        .and_then(|s| s.get("error"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let status = match status_raw {
        Some("completed") => ToolStatus::Success,
        Some("error") if error.to_ascii_lowercase().contains("abort") => ToolStatus::Interrupted,
        Some("error") => ToolStatus::Error,
        Some("running" | "pending") => ToolStatus::Pending,
        _ => ToolStatus::Unknown,
    };
    let (text, pointer) = match state.and_then(|s| s.get("output")).and_then(Value::as_str) {
        Some(output) if status != ToolStatus::Error || error.is_empty() => {
            (output, "/state/output")
        }
        _ if !error.is_empty() => (error, "/state/error"),
        _ => ("", "/state/output"),
    };
    let mut result = tool_result_block(call_id, status, text, || locator.content_ref(pointer));
    if let SessionBlock::ToolResult {
        exit_code,
        duration_ms,
        ..
    } = &mut result
    {
        *exit_code = metadata
            .and_then(|m| m.get("exit"))
            .and_then(Value::as_i64)
            .and_then(|code| i32::try_from(code).ok());
        *duration_ms = state.and_then(time_span);
    }
    blocks.push(result);
}

/// `patch{hash, files[]}`：一步结束时的文件快照，只有文件列表、没有行数（待核实）。
fn push_patch_blocks(blocks: &mut Vec<SessionBlock>, part: &Value) {
    let files: Vec<&str> = part
        .get("files")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    let Some(first) = files.first() else {
        return;
    };
    let id = part
        .get("id")
        .or_else(|| part.get("hash"))
        .and_then(Value::as_str)
        .unwrap_or("patch")
        .to_string();
    let input = part.get("files").cloned().unwrap_or(Value::Null);
    let mut call = tool_call_block(ToolSource::OpenCode, id.clone(), "patch", &input, || None);
    if let SessionBlock::ToolCall {
        title,
        detail,
        diff,
        ..
    } = &mut call
    {
        *title = title_path(first);
        *detail = (files.len() > 1).then(|| format!("{} 个文件", files.len()));
        let mut summary = single_file_diff(first, DiffOp::Update, 0, 0);
        summary.files = files
            .iter()
            .map(|path| single_file_diff(path, DiffOp::Update, 0, 0).files.remove(0))
            .collect();
        *diff = Some(summary);
    }
    blocks.push(call);
    blocks.push(tool_result_block(
        id,
        ToolStatus::Success,
        &files.join("\n"),
        || None,
    ));
}

/// `file{mime, filename, url}`：图片 → Image，其余 → 文件名事件（字段待核实）。
fn push_file_block(blocks: &mut Vec<SessionBlock>, locator: &PartLocator, part: &Value) {
    let mime = part.get("mime").and_then(Value::as_str).unwrap_or("");
    let url = part.get("url").and_then(Value::as_str).unwrap_or("");
    let filename = part
        .get("filename")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty());
    if mime.starts_with("image/") {
        let source = if let Some(path) = url.strip_prefix("file://") {
            Some((ImageSource::LocalFile { path: path.into() }, 0))
        } else if url.starts_with("data:") {
            let base64_len = url.split_once(',').map_or(0, |(_, data)| data.len());
            // data URL 的 base64 在 `,` 之后，P2 解码前需去掉前缀
            locator.content_ref("/url").map(|content| {
                (
                    ImageSource::Inline { content },
                    estimate_base64_size(base64_len),
                )
            })
        } else {
            None
        };
        if let Some((source, size)) = source {
            blocks.push(SessionBlock::Image {
                image: ImageRef {
                    source,
                    media_type: mime.to_string(),
                    size,
                    alt: filename.map(str::to_string),
                },
            });
            return;
        }
    }
    if let Some(text) = filename.or_else(|| (!url.is_empty()).then_some(url)) {
        blocks.push(SessionBlock::event(
            EventKind::Other,
            Some(text.to_string()),
            None,
        ));
    }
}

/// 消息级 `error{name, data{message}}`：中断 → Aborted，其余 → Error。
fn message_error_event(info: &Value) -> Option<SessionBlock> {
    let error = info.get("error")?;
    let name = error.get("name").and_then(Value::as_str).unwrap_or("");
    if name == "MessageAbortedError" {
        return Some(SessionBlock::event(EventKind::Aborted, None, None));
    }
    let text = error
        .pointer("/data/message")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or(name);
    Some(SessionBlock::event(
        EventKind::Error,
        (!text.is_empty()).then(|| text.to_string()),
        None,
    ))
}

/// 消息级 `cost/tokens/modelID/providerID/time/finish` → meta（0 视为缺省）。
fn message_meta(info: &Value) -> Option<MessageMeta> {
    let tokens = info.get("tokens");
    let count = |pointer: &str| {
        tokens
            .and_then(|t| t.pointer(pointer))
            .and_then(Value::as_u64)
            .filter(|n| *n > 0)
    };
    let model_id = info.get("modelID").and_then(Value::as_str);
    let provider_id = info.get("providerID").and_then(Value::as_str);
    let model = match (provider_id, model_id) {
        (Some(p), Some(m)) if !p.is_empty() && !m.is_empty() => Some(format!("{p}/{m}")),
        (_, Some(m)) if !m.is_empty() => Some(m.to_string()),
        _ => None,
    };
    let created = info
        .pointer("/time/created")
        .and_then(parse_timestamp_to_ms);
    let completed = info
        .pointer("/time/completed")
        .and_then(parse_timestamp_to_ms);
    let meta = MessageMeta {
        model,
        input_tokens: count("/input"),
        output_tokens: count("/output"),
        cache_read_tokens: count("/cache/read"),
        cache_write_tokens: count("/cache/write"),
        reasoning_tokens: count("/reasoning"),
        cost_usd: info
            .get("cost")
            .and_then(Value::as_f64)
            .filter(|c| *c > 0.0),
        duration_ms: match (created, completed) {
            (Some(start), Some(end)) if end >= start => end.checked_sub(start).and_then(|n| u64::try_from(n).ok()),
            _ => None,
        },
        stop_reason: info
            .get("finish")
            .and_then(Value::as_str)
            .map(str::to_string),
    };
    (meta != MessageMeta::default()).then_some(meta)
}

/// step-finish 的 tokens：有 `total` 用之，否则把各项相加（本机数据没有 total）。
fn total_tokens(tokens: &Value) -> Option<u64> {
    if let Some(total) = tokens.get("total").and_then(Value::as_u64) {
        return Some(total);
    }
    let sum: u64 = [
        "/input",
        "/output",
        "/reasoning",
        "/cache/read",
        "/cache/write",
    ]
    .iter()
    .filter_map(|p| tokens.pointer(p).and_then(Value::as_u64))
    .fold(0u64, u64::saturating_add);
    (sum > 0).then_some(sum)
}

/// `time{start, end}` → 毫秒时长
fn time_span(value: &Value) -> Option<u64> {
    let start = value.pointer("/time/start").and_then(Value::as_i64)?;
    let end = value.pointer("/time/end").and_then(Value::as_i64)?;
    end.checked_sub(start).and_then(|n| u64::try_from(n).ok())
}

fn part_duration(part: &Value) -> Option<u64> {
    time_span(part).filter(|ms| *ms > 0)
}

fn saturate(n: u64) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

