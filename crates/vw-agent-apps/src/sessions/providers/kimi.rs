//! Kimi committed main-agent history on the selected execution host.
use std::{fs, path::{Path, PathBuf}};
use serde_json::Value;
use vw_api_types::external_session::*;
use crate::sessions::{blocks, utils};
use super::kimi_requests::RequestIndex;

/// Trusted instance resolution supplies KIMI_CODE_HOME, never the gateway's HOME.
pub fn session_roots(data_dir: &Path) -> Vec<PathBuf> { vec![data_dir.join("sessions")] }

fn regular_file(path: &Path) -> bool {
    !path.ancestors().any(|p| fs::symlink_metadata(p).is_ok_and(|m| m.file_type().is_symlink()))
        && fs::symlink_metadata(path).is_ok_and(|m| m.is_file())
}

fn directories(path: &Path) -> Result<Vec<PathBuf>, String> {
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Ok(m) if m.is_dir() && !m.file_type().is_symlink() => {},
        _ => return Err("Invalid Kimi session directory".into()),
    }
    let mut result = Vec::new();
    for entry in fs::read_dir(path).map_err(|_| "Unable to list Kimi sessions")? {
        let entry = entry.map_err(|_| "Unable to inspect Kimi session")?;
        if result.len() >= 100_000 { return Err("Kimi discovery limit exceeded".into()); }
        if entry.file_type().map_err(|_| "Unable to inspect Kimi entry")?.is_dir() { result.push(entry.path()); }
    }
    result.sort();
    Ok(result)
}

pub fn scan_sessions(data_dir: &Path, identity: &ExternalSessionIdentity) -> Result<Vec<SessionMeta>, String> {
    if !data_dir.is_absolute() || identity.provider != SessionProvider::Kimi {
        return Err("Expected an absolute Kimi instance root".into());
    }
    let mut sessions = Vec::new();
    for workspace in directories(&data_dir.join("sessions"))? {
        for dir in directories(&workspace)? {
            if sessions.len() >= 100_000 { return Err("Kimi discovery limit exceeded".into()); }
            let state = [dir.join("state.json"), dir.join("session-meta/state.json")].into_iter().find_map(|path| {
                if !regular_file(&path) || fs::metadata(&path).ok()?.len() > 4 * 1024 * 1024 { return None; }
                fs::read(path).ok().and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
            });
            let Some(state) = state else { continue; };
            if state["archived"].as_bool() == Some(true) || state.pointer("/custom/session_kind").and_then(Value::as_str) == Some("child") { continue; }
            let wire = dir.join("agents/main/wire.jsonl");
            if !regular_file(&wire) { continue; }
            let Some(id) = dir.file_name().and_then(|v| v.to_str()).filter(|id| is_valid_session_id(id)) else { continue; };
            let mut bound = identity.clone();
            bound.session_id = ExternalSessionId(id.into());
            bound.source = SessionSource::File { relative_path: wire.strip_prefix(data_dir).map_err(|_| "Kimi path escaped root")?.to_string_lossy().into_owned() };
            sessions.push(SessionMeta {
                identity: bound,
                capabilities: SessionCapabilities { read: true, delete: true, resume: true, export_markdown: true },
                title: state["title"].as_str().map(str::to_owned),
                summary: state["lastPrompt"].as_str().map(|s| s.chars().take(200).collect()),
                project_dir: state.get("cwd").or_else(|| state.get("workDir")).or_else(|| state.pointer("/custom/cwd")).and_then(Value::as_str).map(str::to_owned),
                created_at: state.get("createdAt").and_then(super::pi_jsonl::parse_timestamp_to_ms),
                last_active_at: state.get("updatedAt").and_then(super::pi_jsonl::parse_timestamp_to_ms),
                source_path: Some(wire.to_string_lossy().into_owned()),
            });
        }
    }
    Ok(sessions)
}

/// Resolve branch switches before projecting messages, so undone/forked turns stay hidden.
struct BranchSegment {
    start: usize,
    end: usize,
    base: Option<(String, usize)>,
}

fn active_records(records: &[(super::pi_jsonl::JsonlSpan, Value)]) -> Result<Vec<usize>, String> {
    let switches: Vec<_> = records
        .iter()
        .enumerate()
        .filter(|(_, (_, record))| record["type"] == "agent.switched")
        .collect();
    let mut active = Vec::new();
    let mut segments = std::collections::HashMap::new();
    let mut branch = "main".to_string();
    let mut start = 0;
    let mut base = None;
    for (index, (_, record)) in switches {
        segments.insert(
            branch,
            BranchSegment {
                start,
                end: index,
                base,
            },
        );
        branch = record
            .get("branch")
            .and_then(Value::as_str)
            .ok_or("Invalid Kimi branch")?
            .into();
        base = Some((
            record
                .pointer("/base/branch")
                .and_then(Value::as_str)
                .ok_or("Invalid Kimi branch base")?
                .into(),
            record
                .pointer("/base/line")
                .and_then(Value::as_u64)
                .ok_or("Invalid Kimi branch line")? try_into().map_err(|_| "Invalid Kimi branch line")?,
        ));
        start = index + 1;
    }
    segments.insert(
        branch.clone(),
        BranchSegment {
            start,
            end: records.len(),
            base,
        },
    );
    let mut chain = Vec::new();
    let mut limit = records.len();
    let mut visited = std::collections::HashSet::new();
    loop {
        if !visited.insert(branch.clone()) {
            return Err("Kimi branch chain contains a cycle".into());
        }
        let segment = segments.get(&branch).ok_or("Unknown Kimi branch")?;
        chain.push((segment.start, segment.end.min(limit)));
        if let Some((parent, line)) = &segment.base {
            if *line > segment.start { return Err("Invalid Kimi branch base line".into()); }
            branch = parent.clone();
            limit = *line;
        } else {
            break;
        }
    }
    for (start, end) in chain.into_iter().rev() {
        active.extend(start..end);
    }
    Ok(active)
}

pub fn load_messages(path: &Path) -> Result<Vec<SessionMessage>, String> {
    if !path.is_absolute() || !regular_file(path) { return Err("Invalid Kimi wire history".into()); }
    if fs::metadata(path).map_err(|_| "Unable to inspect Kimi wire")?.len() > 128 * 1024 * 1024 { return Err("Kimi wire exceeds read limit".into()); }
    let mut records = Vec::new();
    let file = fs::File::open(path).map_err(|error| error.to_string())?;
    let mut lines = super::pi_jsonl::LineSpans::new(std::io::BufReader::new(file));
    while let Some(line) = lines.next_line().map_err(|error| error.to_string())? {
        // Keep physical line positions for agent.switched base.line, including malformed tails.
        records.push((
            line.span,
            serde_json::from_slice::<Value>(line.bytes).unwrap_or(Value::Null),
        ));
    }
    let active = active_records(&records)?;
    let mut requests = RequestIndex::default();
    for index in &active {
        requests.observe(&records[*index].1);
    }
    // Newer files include the committed human projection. Prefer it to the old context stream.
    let has_human = active.iter().any(|index| {
        matches!(
            records[*index].1["type"].as_str(),
            Some("agent.message.appended" | "human.agent.message.appended")
        )
    });
    let mut messages = Vec::new();
    for index in active {
        let (span, record) = &records[index];
        let kind = record["type"].as_str().unwrap_or_default();
        let (entry, pointer) = if has_human
            && matches!(
                kind,
                "agent.message.appended" | "human.agent.message.appended"
            ) {
            (&record["message"], "/message/message")
        } else if !has_human && kind == "context.append_message" {
            (record, "/message")
        } else {
            continue;
        };
        let Some(raw) = record.pointer(pointer) else {
            continue;
        };
        let Some(role @ ("user" | "assistant" | "tool" | "system")) = raw["role"].as_str() else {
            continue;
        };
        let mut message_blocks = Vec::new();
        let reference = |pointer: String| {
            Some(ContentRef::Jsonl {
                offset: span.offset,
                len: span.len,
                pointer,
            })
        };
        if role == "tool" {
            let text = utils::extract_text(&raw["content"]);
            message_blocks.push(blocks::tool_result_block(
                raw["toolCallId"].as_str().unwrap_or_default(),
                ToolStatus::Success,
                &text,
                || reference(format!("{pointer}/content")),
            ));
        } else {
            for (i, part) in raw["content"].as_array().into_iter().flatten().enumerate() {
                match part["type"].as_str() {
                    Some("text") => {
                        if let Some(text) = part["text"].as_str() {
                            message_blocks.push(SessionBlock::text(text));
                        }
                    }
                    Some("think") if part["redacted"].as_bool() != Some(true) => message_blocks.push(blocks::thinking_block(
                        part["think"].as_str().unwrap_or_default(),
                        None,
                        None,
                        || reference(format!("{pointer}/content/{i}/think")),
                    )),
                    _ => {}
                }
            }
            for (i, call) in raw["toolCalls"]
                .as_array()
                .into_iter()
                .flatten()
                .enumerate()
            {
                let input = blocks::parse_arguments(&call["arguments"]);
                message_blocks.push(blocks::tool_call_block(
                    blocks::ToolSource::Generic,
                    call["id"].as_str().unwrap_or_default(),
                    call["name"].as_str().unwrap_or_default(),
                    &input,
                    || reference(format!("{pointer}/toolCalls/{i}/arguments")),
                ));
            }
        }
        if message_blocks.is_empty() {
            continue;
        }
        let mut message = SessionMessage::from_blocks(
            role,
            record.get("time").and_then(super::pi_jsonl::parse_timestamp_to_ms),
            message_blocks,
        );
        let meta = &entry["meta"];
        message.injected = role == "system"
            || matches!(
                meta.pointer("/origin/kind")
                    .or_else(|| raw.pointer("/origin/kind"))
                    .and_then(Value::as_str),
                Some("injection" | "system_trigger" | "retry" | "compaction_summary")
            );
        if let Some(usage) = meta.get("usage") {
            message.meta = Some(MessageMeta {
                model: requests.model(meta).map(str::to_owned),
                input_tokens: usage["inputOther"].as_u64(),
                output_tokens: usage["output"].as_u64(),
                cache_read_tokens: usage["inputCacheRead"].as_u64(),
                cache_write_tokens: usage["inputCacheCreation"].as_u64(),
                duration_ms: requests
                    .get(meta)
                    .and_then(|info| (info.latency_ms > 0).then_some(info.latency_ms as u64)),
                ..Default::default()
            });
        }
        messages.push(message);
    }
    assign_turn_ids(&mut messages);
    Ok(messages)
}

fn is_valid_session_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
}


fn assign_turn_ids(messages: &mut [SessionMessage]) {
    let mut turn = 0u32;
    for message in messages {
        if message.role == "user" && !message.injected && message.blocks.iter().any(|b| matches!(b, SessionBlock::Text { .. })) { turn = turn.saturating_add(1); }
        message.turn_id = Some(format!("t{turn}"));
    }
}
