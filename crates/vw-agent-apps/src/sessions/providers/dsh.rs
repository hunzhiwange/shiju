//! Read-only DeepSeek Harness session browser; native Web UI owns resume/delete.
use crate::sessions::{blocks, utils};
use vw_api_types::external_session::*;
use serde_json::Value;
use std::{
    collections::HashSet,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

const MAX_BYTES: u64 = 128 * 1024 * 1024;

pub fn session_roots(data_dir: &Path) -> Vec<PathBuf> {
    vec![data_dir.join("sessions")]
}

fn generation(path: &Path) -> Option<u64> {
    let name = path.file_name()?.to_str()?;
    name.strip_prefix("session.v")?
        .strip_suffix(".jsonl.zstd")
        .or_else(|| name.strip_prefix("session.v")?.strip_suffix(".jsonl"))?
        .parse()
        .ok()
}

/// Decompress complete independent frames; a partially appended final frame waits for the next scan.
pub(crate) fn read_session(path: &Path) -> Result<Vec<u8>, String> {
    validate_file(path)?;
    let metadata = fs::metadata(path).map_err(|error| error.to_string())?;
    if metadata.len() > MAX_BYTES {
        return Err("DeepSeek Harness session is too large".into());
    }
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    if path.extension().is_none_or(|ext| ext != "zstd") {
        return Ok(bytes);
    }
    let mut output = vec![];
    let mut offset = 0;
    while offset < bytes.len() {
        let frame = &bytes[offset..];
        let size = match zstd::zstd_safe::find_frame_compressed_size(frame) {
            Ok(size) => size,
            Err(_) if frame.len() < 4 || frame.starts_with(&[0x28, 0xb5, 0x2f, 0xfd]) => break,
            Err(_) => return Err("Invalid DeepSeek Harness zstd session".into()),
        };
        let decoder =
            zstd::stream::read::Decoder::new(&frame[..size]).map_err(|error| error.to_string())?;
        decoder
            .take(MAX_BYTES + 1 - output.len() as u64)
            .read_to_end(&mut output)
            .map_err(|error| error.to_string())?;
        if output.len() as u64 > MAX_BYTES {
            return Err("DeepSeek Harness session is too large".into());
        }
        offset += size;
    }
    Ok(output)
}

fn files(root: &Path) -> Vec<PathBuf> {
    let mut result = vec![];
    // Native layout is <project>/<session>/session.vN.jsonl[.zstd]. Do not follow directory links.
    for project in fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .take(100_000)
    {
        if !project.file_type().is_ok_and(|kind| kind.is_dir()) {
            continue;
        }
        for session in fs::read_dir(project.path())
            .into_iter()
            .flatten()
            .flatten()
            .take(100_000)
        {
            if !session.file_type().is_ok_and(|kind| kind.is_dir()) {
                continue;
            }
            let selected = fs::read_dir(session.path())
                .into_iter()
                .flatten()
                .flatten()
                .filter(|file| file.file_type().is_ok_and(|kind| kind.is_file()))
                .map(|file| file.path())
                .filter_map(|path| generation(&path).map(|version| (version, path)))
                .max_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
            if let Some((_, path)) = selected {
                result.push(path);
            }
        }
    }
    result
}

/// `data_dir` comes from the selected execution host's trusted DSH instance.
pub fn scan_sessions(data_dir: &Path, identity: &ExternalSessionIdentity) -> Result<Vec<SessionMeta>, String> {
    if !data_dir.is_absolute() || identity.provider != SessionProvider::Dsh {
        return Err("Expected a DSH instance root and identity".into());
    }
    if data_dir.ancestors().any(|p| fs::symlink_metadata(p).is_ok_and(|m| m.file_type().is_symlink())) {
        return Err("DSH root must not contain symbolic links".into());
    }
    let root = data_dir.join("sessions");
    if fs::symlink_metadata(&root).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err("DSH sessions root must not be a symbolic link".into());
    }
    Ok(files(&root)
        .into_iter()
        .filter_map(|path| {
            let data = read_session(&path).ok()?;
            let mut lines = data.split(|byte| *byte == b'\n');
            let header: Value = serde_json::from_slice(lines.next()?).ok()?;
            if !matches!(header["type"].as_str(), Some("session" | "header")) {
                return None;
            }
            let id = header["id"]
                .as_str()
                .filter(|id| !id.is_empty())?
                .to_string();
            let messages = parse_messages(&data).ok()?;
            let title = messages
                .iter()
                .find(|message| message.role == "user" && !message.injected)
                .map(|message| utils::truncate_summary(&message.content, utils::TITLE_MAX_CHARS));
            let mut bound = identity.clone();
            bound.session_id = ExternalSessionId(id);
            bound.source = SessionSource::File { relative_path: path.strip_prefix(data_dir).ok()?.to_string_lossy().into_owned() };
            Some(SessionMeta {
                identity: bound,
                capabilities: SessionCapabilities { read: true, delete: false, resume: false, export_markdown: true },
                title,
                summary: None,
                project_dir: header["cwd"].as_str().map(str::to_string),
                created_at: header["createdAt"].as_i64(),
                last_active_at: messages.iter().filter_map(|message| message.ts).max(),
                source_path: Some(path.to_string_lossy().into_owned()),
            })
        })
        .collect())
}

pub fn load_messages(path: &Path) -> Result<Vec<SessionMessage>, String> {
    parse_messages(&read_session(path)?)
}

fn parse_messages(data: &[u8]) -> Result<Vec<SessionMessage>, String> {
    let mut messages = vec![];
    let mut seen = HashSet::new();
    let mut offset = 0;
    for line in data.split_inclusive(|byte| *byte == b'\n') {
        let start = offset;
        offset += line.len();
        // An incomplete JSONL tail is still being written.
        if !line.ends_with(b"\n") {
            break;
        }
        let record: Value =
            serde_json::from_slice(line).map_err(|_| "Invalid DeepSeek Harness session JSONL")?;
        // Replacement copies only alter the model-visible surface. The human
        // transcript retains append-origin messages and full tool outputs.
        if record["surfaceOp"]["op"] == "replace" {
            continue;
        }
        let kind = record["type"].as_str().unwrap_or_default();
        let (raw, pointer, role) = match kind {
            "user/message" => (&record["data"], "/data", "user"),
            "assistant/message" => (&record["data"]["message"], "/data/message", "assistant"),
            "tool/result" => (&record["data"]["message"], "/data/message", "tool"),
            _ => continue,
        };
        if let Some(id) = raw["id"].as_str() {
            if !seen.insert(id.to_string()) {
                continue;
            }
        }
        let mut message_blocks = vec![];
        let reference = |pointer: String| {
            Some(ContentRef::Jsonl {
                offset: start as u64,
                len: line.len() as u32,
                pointer,
            })
        };
        if role == "tool" {
            let text = utils::extract_text(&raw["content"]);
            message_blocks.push(blocks::tool_result_block(
                raw["toolCallId"].as_str().unwrap_or_default(),
                if raw["isError"] == true {
                    ToolStatus::Error
                } else {
                    ToolStatus::Success
                },
                &text,
                || reference(format!("{pointer}/content")),
            ));
        } else {
            for (index, part) in raw["content"].as_array().into_iter().flatten().enumerate() {
                match part["type"].as_str() {
                    Some("text") => {
                        if let Some(text) = part["text"].as_str() {
                            message_blocks.push(SessionBlock::text(text));
                        }
                    }
                    Some("thinking" | "reasoning") if part["redacted"].as_bool() != Some(true) => {
                        let field = if part.get("text").is_some() {
                            "text"
                        } else {
                            "thinking"
                        };
                        message_blocks.push(blocks::thinking_block(
                            part[field].as_str().unwrap_or_default(),
                            None,
                            None,
                            || reference(format!("{pointer}/content/{index}/{field}")),
                        ));
                    }
                    Some("tool-call") => {
                        let arguments = blocks::parse_arguments(&part["arguments"]);
                        message_blocks.push(blocks::tool_call_block(
                            blocks::ToolSource::Generic,
                            part["id"].as_str().unwrap_or_default(),
                            part["name"].as_str().unwrap_or_default(),
                            &arguments,
                            || reference(format!("{pointer}/content/{index}/arguments")),
                        ));
                    }
                    _ => {}
                }
            }
        }
        if message_blocks.is_empty() {
            continue;
        }
        let mut message =
            SessionMessage::from_blocks(role, record["time"].as_i64(), message_blocks);
        message.injected = role == "user"
            && raw["source"]["kind"]
                .as_str()
                .is_some_and(|kind| kind != "user");
        let usage = &record["data"]["usage"];
        if role == "assistant" {
            message.meta = Some(MessageMeta {
                model: raw["source"]["model"].as_str().map(str::to_string),
                input_tokens: usage["inputTokens"].as_u64(),
                output_tokens: usage["outputTokens"].as_u64(),
                cache_read_tokens: usage["cacheReadTokens"].as_u64(),
                cache_write_tokens: usage["cacheWriteTokens"].as_u64(),
                ..Default::default()
            });
        }
        messages.push(message);
    }
    let mut turn = 0u32;
    for message in &mut messages {
        if message.role == "user" && !message.injected { turn = turn.saturating_add(1); }
        message.turn_id = Some(format!("t{turn}"));
    }
    Ok(messages)
}


fn validate_file(path: &Path) -> Result<(), String> {
    if !path.is_absolute() || generation(path).is_none()
        || path.ancestors().any(|p| fs::symlink_metadata(p).is_ok_and(|m| m.file_type().is_symlink()))
        || !fs::symlink_metadata(path).is_ok_and(|m| m.is_file()) {
        return Err("Invalid DSH session file".into());
    }
    Ok(())
}

pub fn delete_session(_path: &Path) -> Result<(), String> {
    Err("DSH session deletion is unsupported; use the native Web UI".into())
}
