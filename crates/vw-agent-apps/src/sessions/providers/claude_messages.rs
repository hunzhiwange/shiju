//! Claude transcript parsing, migrated from the pinned upstream snapshot.
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use serde::de::{self, Deserializer, IgnoredAny, MapAccess, SeqAccess, Visitor};
use serde::Deserialize;
use serde_json::Value;
use vw_api_types::external_session::*;
use crate::sessions::blocks::*;
use super::parse_timestamp_to_ms;
include!("claude_jsonl.rs");
include!("claude_blocks.rs");
include!("claude_wire.rs");
pub fn load_messages(path: &Path) -> Result<Vec<SessionMessage>, String> {
    let file = File::open(path).map_err(|e| format!("Failed to open session file: {e}"))?;
    let mut lines = LineSpans::new(BufReader::with_capacity(256 * 1024, file));
    let mut builder = TranscriptBuilder::new(path.with_extension(""));

    // Malformed JSON records are skipped; I/O failures are reported explicitly.
    while let Some(span) = lines.next_line().map_err(|e| format!("Failed to read session record: {e}"))? {
        if span.bytes.is_empty() {
            continue;
        }
        let record: RawRecord = match serde_json::from_slice(span.bytes) {
            Ok(record) => record,
            Err(_) => continue,
        };
        builder.push(record, span.span);
    }

    Ok(builder.finish())
}

/// 解析中的消息；全部记录处理完后再统一投影出 `content`（结果记录会回填前面的 ToolCall）。
struct Draft {
    role: String,
    ts: Option<i64>,
    id: Option<String>,
    injected: bool,
    blocks: Vec<SessionBlock>,
    meta: Option<MessageMeta>,
    /// assistant 的 API `message.id`：Claude Code 把一次回复按内容块拆成多条记录，
    /// 同 id 的相邻记录合并成一条消息
    api_id: Option<String>,
}

/// 已见过的 tool_use 在 `drafts` 中的位置，供结果记录配对与回填。
struct CallSlot {
    draft: usize,
    block: usize,
    ts: Option<i64>,
    name: String,
}

struct TranscriptBuilder {
    /// 会话附属目录 `<sessionId>/`（tool-results 等落盘文件）
    sidecar_dir: PathBuf,
    drafts: Vec<Draft>,
    calls: HashMap<String, CallSlot>,
    /// pr-link 会在每轮结束时重复写入，同一个 URL 只出一次
    pr_urls: HashSet<String>,
}

impl TranscriptBuilder {
    fn new(sidecar_dir: PathBuf) -> Self {
        Self {
            sidecar_dir,
            drafts: Vec::new(),
            calls: HashMap::new(),
            pr_urls: HashSet::new(),
        }
    }

    fn finish(self) -> Vec<SessionMessage> {
        let mut messages: Vec<SessionMessage> = self
            .drafts
            .into_iter()
            .map(|draft| {
                let mut message = SessionMessage::from_blocks(draft.role, draft.ts, draft.blocks);
                message.id = draft.id;
                message.injected = draft.injected;
                message.meta = draft.meta;
                message
            })
            .filter(|message| !message.is_empty())
            .collect();
        assign_turn_ids(&mut messages);
        messages
    }

    fn push(&mut self, record: RawRecord<'_>, line: JsonlSpan) {
        if is_true(&record.is_sidechain) || is_true(&record.is_meta) {
            return;
        }
        let ts = record.timestamp.as_ref().and_then(parse_timestamp_to_ms);

        match record.kind.as_str() {
            Some("system") => self.push_system(&record, ts, line),
            Some("pr-link") => self.push_pr_link(&record, ts),
            Some("user") | Some("assistant") | None => {
                let kind = record.kind.as_str().map(str::to_string);
                let uuid = record.uuid.as_str().map(str::to_string);
                let thinking_ms = record.thinking_duration_ms.as_ref().and_then(as_u64);
                let compact_summary = is_true(&record.is_compact_summary);
                let tool_use_result = record.tool_use_result.0;
                let Some(message) = record.message.0 else {
                    return;
                };
                let role = message
                    .role
                    .as_str()
                    .map(str::to_string)
                    .or(kind)
                    .unwrap_or_else(|| "unknown".to_string());
                if compact_summary {
                    self.push_compact_summary(message.content, ts, line);
                } else if role == "user" {
                    self.push_user(message.content, tool_use_result, uuid, ts, line);
                } else {
                    self.push_assistant(role, message, uuid, thinking_ms, ts, line);
                }
            }
            // attachment、file-history-*、queue-operation、mode、custom-title 等：跳过
            _ => {}
        }
    }

    fn push_draft(
        &mut self,
        role: &str,
        ts: Option<i64>,
        id: Option<String>,
        blocks: Vec<SessionBlock>,
    ) {
        self.drafts.push(Draft {
            role: role.to_string(),
            ts,
            id,
            injected: false,
            blocks,
            meta: None,
            api_id: None,
        });
    }

    fn push_event(
        &mut self,
        ts: Option<i64>,
        kind: EventKind,
        text: Option<String>,
        url: Option<String>,
    ) {
        self.push_draft(
            "system",
            ts,
            None,
            vec![SessionBlock::event(kind, text, url)],
        );
    }

    // ── system / pr-link ──

    fn push_system(&mut self, record: &RawRecord<'_>, ts: Option<i64>, line: JsonlSpan) {
        match record.subtype.as_str() {
            Some("stop_hook_summary") => {
                let errors: Vec<String> = match &record.hook_errors {
                    Some(Value::Array(items)) => items
                        .iter()
                        .map(|item| match item {
                            Value::String(s) => s.trim().to_string(),
                            other => other.to_string(),
                        })
                        .filter(|s| !s.is_empty())
                        .collect(),
                    _ => Vec::new(),
                };
                if !errors.is_empty() {
                    self.push_event(ts, EventKind::Hook, Some(errors.join("\n")), None);
                }
            }
            Some("compact_boundary") => {
                let text = record
                    .content
                    .as_str()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .unwrap_or("Conversation compacted");
                self.push_event(ts, EventKind::Compaction, Some(text.to_string()), None);
            }
            // 新版 Claude Code 把本地斜杠命令记成 system.local_command
            Some("local_command") => {
                if let Some(content) = record.content.as_str() {
                    let uuid = record.uuid.as_str().map(str::to_string);
                    self.push_user_texts(
                        vec![(Cow::Borrowed(content), line.content_ref("/content"))],
                        Vec::new(),
                        Vec::new(),
                        uuid,
                        ts,
                    );
                }
            }
            // turn_duration、api_error、informational、away_summary…：不展示
            _ => {}
        }
    }

    fn push_pr_link(&mut self, record: &RawRecord<'_>, ts: Option<i64>) {
        let Some(url) = record.pr_url.as_str().filter(|u| !u.trim().is_empty()) else {
            return;
        };
        if !self.pr_urls.insert(url.to_string()) {
            return;
        }
        let text = match record.pr_number.as_ref() {
            Some(Value::Number(n)) => format!("PR #{n}"),
            Some(Value::String(s)) if !s.is_empty() => format!("PR #{s}"),
            _ => "PR".to_string(),
        };
        self.push_event(ts, EventKind::PrLink, Some(text), Some(url.to_string()));
    }

    /// `/compact` 之后的摘要记录：并入紧邻的 compact_boundary 事件，没有则单独成一条。
    /// 摘要通常有数 KB，只放预览，全文按 `/message/content` 引用取。
    fn push_compact_summary(&mut self, content: RawContent<'_>, ts: Option<i64>, line: JsonlSpan) {
        let summary = content_text(&content);
        if summary.trim().is_empty() {
            return;
        }
        let block = summary_event_block(EventKind::Compaction, &summary, || {
            Some(line.content_ref("/message/content"))
        });
        if let Some(last) = self.drafts.last_mut() {
            if let [event @ SessionBlock::Event {
                kind: EventKind::Compaction,
                ..
            }] = last.blocks.as_mut_slice()
            {
                *event = block;
                return;
            }
        }
        self.push_draft("system", ts, None, vec![block]);
    }

    // ── assistant ──

    fn push_assistant(
        &mut self,
        role: String,
        message: RawMessage<'_>,
        uuid: Option<String>,
        thinking_ms: Option<u64>,
        ts: Option<i64>,
        line: JsonlSpan,
    ) {
        let mut blocks = Vec::new();
        // (块下标, tool_use id, 工具名)
        let mut calls: Vec<(usize, String, String)> = Vec::new();
        match message.content {
            RawContent::Text(text) => {
                if !text.trim().is_empty() {
                    blocks.push(SessionBlock::text(text.into_owned()));
                }
            }
            RawContent::Items(items) => {
                for (i, item) in items.into_iter().enumerate() {
                    let Some(item) = item.0 else { continue };
                    match item.kind.as_str().unwrap_or_default() {
                        "text" => {
                            if let Some(text) = item.text.0.filter(|t| !t.trim().is_empty()) {
                                blocks.push(SessionBlock::text(text.into_owned()));
                            }
                        }
                        kind @ ("thinking" | "redacted_thinking") => {
                            if let Some(block) = thinking_block(&item, kind, i, thinking_ms, line) {
                                blocks.push(block);
                            }
                        }
                        "tool_use" | "server_tool_use" => {
                            let block = tool_call_block(item, i, line);
                            if let SessionBlock::ToolCall { id, raw_name, .. } = &block {
                                if !id.is_empty() {
                                    calls.push((blocks.len(), id.clone(), raw_name.clone()));
                                }
                            }
                            blocks.push(block);
                        }
                        _ => {}
                    }
                }
            }
            RawContent::None => {}
        }
        if blocks.is_empty() {
            return;
        }

        let meta = message_meta(&message.model, message.usage.as_ref(), &message.stop_reason);
        let api_id = message.id.as_str().map(str::to_string);
        let mergeable = self
            .drafts
            .last()
            .is_some_and(|last| last.role == role && api_id.is_some() && last.api_id == api_id);
        let (draft, base) = if mergeable {
            let last = self.drafts.last_mut().expect("checked above");
            let base = last.blocks.len();
            last.blocks.extend(blocks);
            merge_meta(&mut last.meta, meta);
            (self.drafts.len() - 1, base)
        } else {
            self.drafts.push(Draft {
                role,
                ts,
                id: uuid,
                injected: false,
                blocks,
                meta,
                api_id,
            });
            (self.drafts.len() - 1, 0)
        };
        for (index, id, name) in calls {
            self.calls.insert(
                id,
                CallSlot {
                    draft,
                    block: base + index,
                    ts,
                    name,
                },
            );
        }
    }

    // ── user（提问 / 工具结果 / 注入内容）──

    fn push_user(
        &mut self,
        content: RawContent<'_>,
        tool_use_result: Option<ToolUseResult>,
        uuid: Option<String>,
        ts: Option<i64>,
        line: JsonlSpan,
    ) {
        let mut results = Vec::new();
        let mut texts = Vec::new();
        let mut images = Vec::new();
        match content {
            RawContent::Text(text) => texts.push((text, line.content_ref("/message/content"))),
            RawContent::Items(items) => {
                let result_count = items
                    .iter()
                    .filter(|item| {
                        item.0
                            .as_ref()
                            .is_some_and(|i| i.kind.as_str() == Some("tool_result"))
                    })
                    .count();
                // 顶层 toolUseResult 只描述一个结果；多结果记录不知道属于谁，不用
                let tool_use_result = tool_use_result.filter(|_| result_count == 1);
                for (i, item) in items.into_iter().enumerate() {
                    let Some(item) = item.0 else { continue };
                    match item.kind.as_str().unwrap_or_default() {
                        "tool_result" => results.push(self.tool_result_block(
                            item,
                            i,
                            tool_use_result.as_ref(),
                            ts,
                            line,
                        )),
                        "text" => {
                            if let Some(text) = item.text.0 {
                                texts.push((
                                    text,
                                    line.content_ref(format!("/message/content/{i}/text")),
                                ));
                            }
                        }
                        "image" => {
                            if let Some(image) = image_ref(
                                item.source.0.as_ref(),
                                format!("/message/content/{i}/source/data"),
                                line,
                            ) {
                                images.push(SessionBlock::Image { image });
                            }
                        }
                        _ => {}
                    }
                }
            }
            RawContent::None => {}
        }
        self.push_user_texts(texts, results, images, uuid, ts);
    }

    /// 按 §4.1 给用户文本归类：中断 / 斜杠命令 / `!` 命令 / 注入内容 / 普通提问。
    /// `texts` 每项带上它在源记录里的引用，超长注入文本只放预览 + 引用。
    fn push_user_texts(
        &mut self,
        texts: Vec<(Cow<'_, str>, ContentRef)>,
        mut blocks: Vec<SessionBlock>,
        images: Vec<SessionBlock>,
        uuid: Option<String>,
        ts: Option<i64>,
    ) {
        let has_results = !blocks.is_empty();
        let mut injected = false;

        let injected_blocks = |texts: &[(Cow<'_, str>, ContentRef)]| -> Vec<SessionBlock> {
            texts
                .iter()
                .map(|(text, full)| large_text_block(text.to_string(), || Some(full.clone())))
                .collect()
        };
        if let Some(first) = texts.iter().map(|(t, _)| t.trim()).find(|t| !t.is_empty()) {
            if first.starts_with("[Request interrupted by user") {
                blocks.push(SessionBlock::event(
                    EventKind::Aborted,
                    Some(first.to_string()),
                    None,
                ));
            } else if let Some(command) = slash_command(first) {
                blocks.push(SessionBlock::event(
                    EventKind::SlashCommand,
                    Some(command),
                    None,
                ));
            } else if first.starts_with("<bash-input>") {
                blocks.extend(user_bash_blocks(first, uuid.as_deref()));
            } else if is_injected_text(first) {
                // 与工具结果同记录的注入文本直接丢弃；单独成条时整条标 injected、保留原文
                if !has_results {
                    injected = true;
                    blocks.extend(injected_blocks(&texts));
                }
            } else {
                let stripped: Vec<String> = texts
                    .iter()
                    .map(|(t, _)| strip_system_reminders(t))
                    .filter(|t| !t.trim().is_empty())
                    .collect();
                if !stripped.is_empty() {
                    blocks.extend(stripped.into_iter().map(SessionBlock::text));
                } else if !has_results && images.is_empty() {
                    // 全是 <system-reminder>：整条注入，保留原文
                    injected = true;
                    blocks.extend(injected_blocks(&texts));
                }
            }
        }
        blocks.extend(images);
        if blocks.is_empty() {
            return;
        }

        let all_results = blocks
            .iter()
            .all(|b| matches!(b, SessionBlock::ToolResult { .. }));
        let role = if all_results { "tool" } else { "user" };
        self.push_draft(role, ts, uuid, blocks);
        if let Some(last) = self.drafts.last_mut() {
            last.injected = injected;
        }
    }

    fn tool_result_block(
        &mut self,
        item: RawItem<'_>,
        index: usize,
        tool_use_result: Option<&ToolUseResult>,
        ts: Option<i64>,
        line: JsonlSpan,
    ) -> SessionBlock {
        let call_id = item.tool_use_id.as_str().unwrap_or_default().to_string();
        let base = format!("/message/content/{index}/content");
        let mut images = Vec::new();
        let (text, pointer): (Cow<'_, str>, Option<String>) = match item.content {
            RawContent::Text(text) => (text, Some(base)),
            RawContent::Items(parts) => {
                let mut texts: Vec<(usize, Cow<'_, str>)> = Vec::new();
                for (j, part) in parts.into_iter().enumerate() {
                    let Some(part) = part.0 else { continue };
                    match part.kind.as_str().unwrap_or_default() {
                        "text" => {
                            if let Some(text) = part.text.0 {
                                texts.push((j, text));
                            }
                        }
                        "image" => {
                            if let Some(image) = image_ref(
                                part.source.0.as_ref(),
                                format!("{base}/{j}/source/data"),
                                line,
                            ) {
                                images.push(image);
                            }
                        }
                        _ => {}
                    }
                }
                match texts.len() {
                    0 => (Cow::Borrowed(""), None),
                    // 只有一段文本时直接指向字符串，P2 不用再拼接
                    1 => {
                        let (j, text) = texts.pop().expect("len checked");
                        (text, Some(format!("{base}/{j}/text")))
                    }
                    _ => {
                        let joined = texts
                            .iter()
                            .map(|(_, t)| t.as_ref())
                            .collect::<Vec<_>>()
                            .join("\n");
                        (Cow::Owned(joined), Some(base))
                    }
                }
            }
            RawContent::None => (Cow::Borrowed(""), None),
        };
        let text = if text.contains(SYSTEM_REMINDER_OPEN) {
            Cow::Owned(strip_system_reminders(&text))
        } else {
            text
        };
        let p = preview(&text);

        let is_error = item.is_error.as_ref().and_then(Value::as_bool) == Some(true);
        let interrupted = tool_use_result
            .and_then(|r| r.interrupted.as_ref())
            .and_then(Value::as_bool)
            == Some(true);
        let status = if interrupted {
            ToolStatus::Interrupted
        } else if is_error {
            ToolStatus::Error
        } else {
            ToolStatus::Success
        };

        let slot = self.calls.get(&call_id);
        let is_shell = slot.is_some_and(|s| s.name == "Bash");
        // 退出码与耗时只对 Bash 有意义（其余工具的「结果时间 - 调用时间」多是排队与思考时间）
        let exit_code = parse_exit_code(&text).or_else(|| {
            let background = tool_use_result.is_some_and(|r| r.background_task_id.is_some());
            (is_shell && status == ToolStatus::Success && !background).then_some(0)
        });
        let duration_ms = match (is_shell, slot.and_then(|s| s.ts), ts) {
            (true, Some(start), Some(end)) if end >= start => Some((end - start) as u64),
            _ => None,
        };

        let saved_path = tool_use_result
            .and_then(|r| r.persisted_output_path.as_ref())
            .and_then(Value::as_str)
            .filter(|p| !p.is_empty())
            .map(str::to_string);
        let full = if p.truncated {
            saved_path
                .as_deref()
                .and_then(|path| self.sidecar_rel_path(path))
                .map(|rel_path| ContentRef::Sidecar { rel_path })
                .or_else(|| pointer.map(|ptr| line.content_ref(ptr)))
        } else {
            None
        };

        self.backfill_call(&call_id, status, tool_use_result);

        SessionBlock::ToolResult {
            call_id,
            status,
            preview: p.text,
            total_len: p.total_len,
            line_count: p.line_count,
            truncated: p.truncated,
            full,
            exit_code,
            duration_ms,
            images,
            saved_path,
        }
    }

    /// 结果到来时回填对应 ToolCall：Edit/Write 用 `structuredPatch` 修正 diff，
    /// 失败的改动去掉 diff；Agent 用实际模型补 detail。
    fn backfill_call(&mut self, call_id: &str, status: ToolStatus, result: Option<&ToolUseResult>) {
        let Some(slot) = self.calls.get(call_id) else {
            return;
        };
        let Some(SessionBlock::ToolCall {
            raw_name,
            detail,
            diff,
            ..
        }) = self
            .drafts
            .get_mut(slot.draft)
            .and_then(|d| d.blocks.get_mut(slot.block))
        else {
            return;
        };

        match raw_name.as_str() {
            "Edit" | "MultiEdit" | "Write" => {
                if status != ToolStatus::Success {
                    *diff = None;
                    return;
                }
                let Some(result) = result else { return };
                let op = match result.kind.as_ref().and_then(Value::as_str) {
                    Some("create") => Some(DiffOp::Add),
                    Some("update") => Some(DiffOp::Update),
                    _ => None,
                };
                let path = result
                    .file_path
                    .as_ref()
                    .and_then(Value::as_str)
                    .map(str::to_string);
                let patch = result.structured_patch.filter(|p| p.added + p.removed > 0);
                if op.is_none() && patch.is_none() {
                    return;
                }
                let summary = diff.get_or_insert_with(|| DiffSummary {
                    files: vec![DiffFile {
                        path: String::new(),
                        op: DiffOp::Update,
                        added: 0,
                        removed: 0,
                    }],
                    added: 0,
                    removed: 0,
                    full: None,
                });
                if let Some(file) = summary.files.first_mut() {
                    if let Some(op) = op {
                        file.op = op;
                    }
                    if let Some(path) = path {
                        file.path = path;
                    }
                    if let Some(patch) = patch {
                        file.added = patch.added;
                        file.removed = patch.removed;
                    }
                    summary.added = file.added;
                    summary.removed = file.removed;
                }
            }
            "Agent" | "Task" if detail.is_none() => {
                *detail = result
                    .and_then(|r| r.resolved_model.as_ref())
                    .and_then(Value::as_str)
                    .filter(|m| !m.is_empty())
                    .map(str::to_string);
            }
            _ => {}
        }
    }

    /// `persistedOutputPath` 在会话附属目录内时，转成相对路径（`ContentRef::Sidecar`）。
    fn sidecar_rel_path(&self, saved: &str) -> Option<String> {
        Path::new(saved)
            .strip_prefix(&self.sidecar_dir)
            .ok()
            .filter(|rel| rel.components().all(|part| matches!(part, std::path::Component::Normal(_))))
            .map(|rel| rel.to_string_lossy().replace('\\', "/"))
            .filter(|rel| !rel.is_empty())
    }
}

