//! Structured Codex rollout messages; encrypted fields are never retained or decoded.
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use serde::Deserialize;
use serde_json::Value;
use vw_api_types::external_session::*;
use crate::sessions::rollout as codex_rollout_file;
use crate::sessions::blocks::{
    first_string_field, large_text_block, normalize_tool, preview, preview_chars,
    refine_shell_kind, summary_event_block, title_agent, title_ask, title_mcp, title_other,
    title_path, title_read, title_shell, title_todo, title_web, ToolSource, THINKING_PREVIEW_CHARS,
};
use super::codex_items::{
    apply_patch_rejected, command_from_value, diff_kind, diff_summary, diff_title, exec_title,
    image_from_url, local_image, opt_str, parse_apply_patch, parse_exec_script,
    parse_legacy_shell_output, same_command, status_from_str, strip_output_header,
    web_action_title, CallSpec, CommandInfo, Contents, FileChangeInfo, ItemRecord, RawItem,
    ResultSpec, Str,
};
use super::pi_jsonl::{JsonlSpan, LineSpans};
pub fn load_messages(path: &Path) -> Result<Vec<SessionMessage>, String> {
    let reader = codex_rollout_file::open_reader(path)
        .map_err(|e| format!("Failed to open session file: {e}"))?;
    let mut lines = LineSpans::new(reader);
    let mut parser = RolloutParser::default();
    while let Some(line) = lines
        .next_line()
        .map_err(|e| format!("Failed to read session file: {e}"))?
    {
        let mut bytes = line.bytes;
        while let [rest @ .., b'\n' | b'\r'] = bytes {
            bytes = rest;
        }
        if !bytes.is_empty() {
            parser.process_line(bytes, line.span);
        }
    }
    Ok(parser.finish())
}

// ─── 行结构（只声明用得到的字段，其余跳过不分配）──────────────────────────

#[derive(Deserialize)]
struct Line<'a, P> {
    #[serde(default, borrow)]
    timestamp: Option<Str<'a>>,
    payload: P,
}

#[derive(Deserialize)]
struct Head<'a> {
    #[serde(rename = "type", default, borrow)]
    ty: Option<Str<'a>>,
    #[serde(default, borrow)]
    payload: Option<PayloadHead<'a>>,
}

#[derive(Deserialize)]
struct PayloadHead<'a> {
    #[serde(rename = "type", default, borrow)]
    ty: Option<Str<'a>>,
}

#[derive(Deserialize)]
struct MessagePayload<'a> {
    #[serde(default, borrow)]
    id: Option<Str<'a>>,
    #[serde(default, borrow)]
    role: Option<Str<'a>>,
    #[serde(default, borrow)]
    content: Option<Contents<'a>>,
    #[serde(default, borrow)]
    phase: Option<Str<'a>>,
}

#[derive(Deserialize)]
struct ReasoningPayload<'a> {
    #[serde(default, borrow)]
    id: Option<Str<'a>>,
    #[serde(default, borrow)]
    summary: Option<Contents<'a>>,
    #[serde(default, borrow)]
    content: Option<Contents<'a>>,
}

#[derive(Deserialize)]
struct FunctionCallPayload<'a> {
    #[serde(default, borrow)]
    id: Option<Str<'a>>,
    #[serde(default, borrow)]
    name: Option<Str<'a>>,
    #[serde(default, borrow)]
    namespace: Option<Str<'a>>,
    #[serde(default, borrow)]
    arguments: Option<Str<'a>>,
    #[serde(default, borrow)]
    call_id: Option<Str<'a>>,
}

#[derive(Deserialize)]
struct CustomCallPayload<'a> {
    #[serde(default, borrow)]
    id: Option<Str<'a>>,
    #[serde(default, borrow)]
    name: Option<Str<'a>>,
    #[serde(default, borrow)]
    input: Option<Str<'a>>,
    #[serde(default, borrow)]
    call_id: Option<Str<'a>>,
}

#[derive(Deserialize)]
struct OutputPayload<'a> {
    #[serde(default, borrow)]
    id: Option<Str<'a>>,
    #[serde(default, borrow)]
    call_id: Option<Str<'a>>,
    #[serde(default, borrow)]
    output: Option<Contents<'a>>,
}

#[derive(Deserialize)]
struct ActionPayload<'a> {
    #[serde(default, borrow)]
    id: Option<Str<'a>>,
    #[serde(default, borrow)]
    call_id: Option<Str<'a>>,
    #[serde(default, borrow)]
    status: Option<Str<'a>>,
    #[serde(default)]
    action: Option<Value>,
}

#[derive(Deserialize)]
struct AgentMessagePayload<'a> {
    #[serde(default, borrow)]
    id: Option<Str<'a>>,
    #[serde(default, borrow)]
    author: Option<Str<'a>>,
    #[serde(default, borrow)]
    content: Option<Contents<'a>>,
}

#[derive(Deserialize)]
struct TurnContextPayload<'a> {
    #[serde(default, borrow)]
    turn_id: Option<Str<'a>>,
    #[serde(default, borrow)]
    model: Option<Str<'a>>,
}

#[derive(Deserialize, Default, Clone, Copy)]
struct TokenUsage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    cached_input_tokens: u64,
    #[serde(default)]
    cache_write_input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
    #[serde(default)]
    reasoning_output_tokens: u64,
}

impl TokenUsage {
    fn add(&mut self, other: &TokenUsage) {
        self.input_tokens += other.input_tokens;
        self.cached_input_tokens += other.cached_input_tokens;
        self.cache_write_input_tokens += other.cache_write_input_tokens;
        self.output_tokens += other.output_tokens;
        self.reasoning_output_tokens += other.reasoning_output_tokens;
    }
}

#[derive(Deserialize)]
struct UsageRecordPayload<'a> {
    #[serde(default, borrow)]
    turn_id: Option<Str<'a>>,
    #[serde(default)]
    turn_token_usage: Option<TokenUsage>,
}

#[derive(Deserialize)]
struct CompactedPayload<'a> {
    #[serde(default, borrow)]
    message: Option<Str<'a>>,
}

#[derive(Deserialize)]
struct ThreadSettings<'a> {
    #[serde(default, borrow)]
    model: Option<Str<'a>>,
}

#[derive(Deserialize)]
struct TokenInfo {
    #[serde(default)]
    last_token_usage: Option<TokenUsage>,
}

/// item_completed 以外的 event_msg（task_started / task_complete / turn_aborted /
/// thread_settings_applied / token_count）
#[derive(Deserialize)]
struct EventPayload<'a> {
    #[serde(default, borrow)]
    turn_id: Option<Str<'a>>,
    #[serde(default)]
    reason: Option<Value>,
    #[serde(default, borrow)]
    thread_settings: Option<ThreadSettings<'a>>,
    #[serde(default)]
    info: Option<TokenInfo>,
}

#[derive(Deserialize)]
struct ItemCompletedPayload<'a> {
    #[serde(borrow)]
    item: RawItem<'a>,
}

fn owned(value: &Option<Str<'_>>) -> Option<String> {
    opt_str(value).map(str::to_string)
}

fn parse_ts(raw: Option<&Str<'_>>) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(raw?.as_str())
        .ok()
        .map(|dt| dt.timestamp_millis())
}

/// 解析整行，只取 `timestamp` 与指定形状的 `payload`
fn parse_line<'a, P: Deserialize<'a>>(line: &'a [u8]) -> Option<(Option<i64>, P)> {
    let parsed: Line<'a, P> = serde_json::from_slice(line).ok()?;
    Some((parse_ts(parsed.timestamp.as_ref()), parsed.payload))
}

/// 从行首嗅探 `(顶层 type, payload.type)`，避免为分派把大行完整解析两遍。
/// Codex 写出的行形如 `{"timestamp":…,"ordinal":N,"type":"response_item","payload":{"type":"…",…`；
/// 键序不符时返回 `None`，由调用方退回完整解析。
fn sniff_types(line: &[u8]) -> Option<(&str, Option<&str>)> {
    let head = &line[..line.len().min(SNIFF_BYTES)];
    let head = match std::str::from_utf8(head) {
        Ok(head) => head,
        Err(e) => std::str::from_utf8(&head[..e.valid_up_to()]).ok()?,
    };
    let payload_at = head.find("\"payload\":")?;
    let top = quoted_value_after(&head[..payload_at], "\"type\":\"")?;
    let sub = head[payload_at + "\"payload\":".len()..]
        .trim_start()
        .strip_prefix('{')
        .map(str::trim_start)
        .and_then(|p| p.strip_prefix("\"type\":\""))
        .and_then(|p| p.split_once('"'))
        .map(|(ty, _)| ty);
    Some((top, sub))
}

fn quoted_value_after<'s>(haystack: &'s str, key: &str) -> Option<&'s str> {
    let start = haystack.find(key)? + key.len();
    haystack[start..].split_once('"').map(|(value, _)| value)
}

// ─── 状态机 ──────────────────────────────────────────────────────────────

/// 工具调用的来源形态，决定哪些结构化项能挂上来
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CallFlavor {
    /// custom_tool_call(exec)：一段 JS，可能调用多条命令 / MCP / apply_patch
    Exec,
    /// function_call(exec_command/shell)、local_shell_call
    Shell,
    ApplyPatch,
    Mcp,
    Other,
}

struct PendingCall {
    msg_idx: usize,
    block_idx: usize,
    flavor: CallFlavor,
    /// 调用参数里能看到的命令，用来认领 CommandExecution
    cmds: Vec<String>,
    items: Vec<ItemRecord>,
}

/// 工具输出（去掉输出头后的正文 + 头里的结构化信息）
struct OutputInfo {
    text: String,
    full: Option<ContentRef>,
    exit_code: Option<i32>,
    duration_ms: Option<u64>,
    failed: bool,
    images: Vec<ImageRef>,
}

impl OutputInfo {
    fn from_contents(contents: Option<&Contents<'_>>, span: JsonlSpan) -> Self {
        let mut info = OutputInfo {
            text: String::new(),
            full: None,
            exit_code: None,
            duration_ms: None,
            failed: false,
            images: Vec::new(),
        };
        match contents {
            None => {}
            Some(Contents::Text(text)) => {
                let text = text.as_str();
                if let Some((body, exit_code, duration_ms)) = parse_legacy_shell_output(text) {
                    info.text = body;
                    info.exit_code = exit_code;
                    info.duration_ms = duration_ms;
                } else {
                    let header = strip_output_header(text);
                    info.text = header.body.to_string();
                    info.exit_code = header.exit_code;
                    info.duration_ms = header.duration_ms;
                    info.failed = header.failed;
                }
                info.full = Some(span.content_ref("/payload/output"));
            }
            Some(Contents::Items(items)) => {
                let mut pieces: Vec<(usize, &str)> = Vec::new();
                for (i, item) in items.iter().enumerate() {
                    match item.kind() {
                        "input_image" | "image" => {
                            if let Some(url) = opt_str(&item.image_url) {
                                let pointer = format!("/payload/output/{i}/image_url");
                                info.images.extend(image_from_url(url, span, pointer));
                            }
                        }
                        _ => {
                            let Some(text) = opt_str(&item.text) else {
                                continue;
                            };
                            let header = strip_output_header(text);
                            info.exit_code = info.exit_code.or(header.exit_code);
                            info.duration_ms = info.duration_ms.or(header.duration_ms);
                            info.failed |= header.failed;
                            if !header.body.is_empty() {
                                pieces.push((i, header.body));
                            }
                        }
                    }
                }
                info.full = Some(span.content_ref(match pieces.as_slice() {
                    [(i, _)] => format!("/payload/output/{i}/text"),
                    _ => "/payload/output".to_string(),
                }));
                info.text = pieces.iter().map(|(_, t)| *t).collect();
            }
        }
        info
    }
}

#[derive(Default)]
struct RolloutParser {
    messages: Vec<SessionMessage>,
    turn_id: Option<String>,
    /// 最近一次看到的模型（turn_context / thread_settings_applied）
    last_model: Option<String>,
    /// 模型变了，等下一轮开始时输出 ModelChange 事件
    pending_model_event: Option<(String, Option<i64>)>,
    turn_models: HashMap<String, String>,
    turn_usage: HashMap<String, TokenUsage>,
    usage_from_records: bool,
    /// AgentMessage.phase（commentary / final_answer），按消息 id
    phases: HashMap<String, String>,
    calls: HashMap<String, PendingCall>,
    /// 已经输出结果的调用 id。Codex 0.119–0.128 的写入顺序是「调用 → 输出 →
    /// item_completed」，迟到的 item_completed 按 id 命中这里时只回填退出码 / 耗时，
    /// 不再单独成一个步骤（否则同一条命令会显示两次）
    finished: HashSet<String>,
    /// 还没等到输出的调用（按出现顺序）
    open: Vec<String>,
    /// 并行 exec 时归属不明的 CommandExecution，等输出到达时按内容认领
    pool: Vec<CommandInfo>,
    /// 紧跟的 web_search_call 没有 id 时借用 WebSearch 项的 id
    last_web_search_id: Option<String>,
    /// 刚输出的 compaction 事件（compacted / compaction / ContextCompaction 去重）
    last_compaction: Option<usize>,
}

#[path = "codex_parser_1.rs"]
mod parser_1;
#[path = "codex_parser_2.rs"]
mod parser_2;
#[path = "codex_parser_3.rs"]
mod parser_3;
#[path = "codex_parser_4.rs"]
mod parser_4;

enum Claim {
    Call(String),
    Pool,
    None,
}

impl From<Option<String>> for Claim {
    fn from(call_id: Option<String>) -> Self {
        call_id.map_or(Claim::None, Claim::Call)
    }
}

/// 结果状态：CommandExecution 的状态优先，其次输出头（Script failed / 非零退出码）
fn output_status(out: &OutputInfo, command: Option<&CommandInfo>) -> ToolStatus {
    let mut status = match command.map(|c| c.status) {
        Some(ToolStatus::Unknown | ToolStatus::Pending) | None => ToolStatus::Success,
        Some(status) => status,
    };
    let exit_code = command.and_then(|c| c.exit_code).or(out.exit_code);
    if status == ToolStatus::Success && (out.failed || exit_code.is_some_and(|c| c != 0)) {
        status = ToolStatus::Error;
    }
    status
}

/// 整段被同一个 XML 风格标签包住（`<recommended_plugins>…</recommended_plugins>` 之类）：
/// 是客户端注入的上下文，不是用户手打的提问
fn is_wrapped_in_tag(text: &str) -> bool {
    let Some(rest) = text.strip_prefix('<') else {
        return false;
    };
    let name: &str = rest
        .split(|c: char| c == '>' || c.is_whitespace())
        .next()
        .unwrap_or_default();
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        && text.ends_with(&format!("</{name}>"))
}

/// Codex 给图片包的 `<image name=…>` / `</image>` 标签行
fn is_image_wrapper(text: &str) -> bool {
    text == "</image>"
        || (text.starts_with("<image ") && text.ends_with('>') && !text.contains('\n'))
}

