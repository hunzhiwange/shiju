//! Bounded lazy text reads from a server-validated session locator.
use std::{fs::File, io::Read, path::{Component, Path, PathBuf}};
use serde::{Serialize, Deserialize};
use serde_json::Value;
use vw_api_types::external_session::{ContentRef, SessionProvider};
use super::source::ValidatedSource;

pub const MAX_TEXT_BYTES: u64 = 32 * 1024 * 1024;
pub const MAX_PAGE_CHARS: u32 = 512 * 1024;
const STALE: &str = "会话已更新，请重新读取";
const NOT_TEXT: &str = "引用的内容不是文本";
const OUTSIDE: &str = "引用不属于当前会话";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockContent {
    pub text: String,
    pub total_len: u32,
    pub truncated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<u32>,
}

pub fn paginate(full: &str, offset: Option<u32>, limit: Option<u32>) -> Result<BlockContent, String> {
    if full.len() as u64 > MAX_TEXT_BYTES { return Err("内容超过32MiB".into()); }
    let offset = offset.unwrap_or(0) as usize;
    let total = full.chars().count();
    if offset > total { return Err("字符偏移超出内容范围".into()); }
    let limit = limit.unwrap_or(MAX_PAGE_CHARS).clamp(1, MAX_PAGE_CHARS) as usize;
    let text: String = full.chars().skip(offset).take(limit).collect();
    let end = (offset + limit).min(total);
    Ok(BlockContent { text, total_len: total as u32, truncated: end < total,
        next_offset: (end < total).then_some(end as u32) })
}

pub fn resolve_content_ref(source: &ValidatedSource, content: &ContentRef) -> Result<String, String> {
    if source.record_id().is_some() { return Err(OUTSIDE.into()); }
    let text = match content {
        ContentRef::Jsonl { offset, len, pointer } => {
            let path = if source.identity().provider == SessionProvider::Grokbuild {
                safe_join(source.root(), source.path().parent().ok_or(OUTSIDE)?.strip_prefix(source.root()).map_err(|_| OUTSIDE)?.join("chat_history.jsonl").to_str().ok_or(OUTSIDE)?)?
            } else { checked_path(source.path())? };
            let line = read_jsonl_line(&path, *offset, *len)?;
            let value: Value = serde_json::from_slice(&line).map_err(|_| STALE)?;
            extract_text(&value, pointer)?
        }
        ContentRef::File { rel_path, pointer } => {
            let path = safe_join(source.root(), rel_path)?;
            let bytes = read_limited(&path)?;
            let value: Value = if source.identity().provider == SessionProvider::Gemini {
                super::providers::gemini::parse_session_document(std::str::from_utf8(&bytes).map_err(|_| NOT_TEXT)?).ok_or(STALE)?
            } else { serde_json::from_slice(&bytes).map_err(|_| STALE)? };
            if path != source.path() {
                validate_opencode_part(source, &path, &value)?;
            }
            if source.identity().provider == SessionProvider::Gemini && pointer.ends_with("/thoughts") {
                super::providers::gemini::format_thoughts(value.pointer(pointer).ok_or(STALE)?).ok_or(NOT_TEXT)?.text
            } else { extract_text(&value, pointer)? }
        }
        ContentRef::Sidecar { rel_path } => {
            if source.identity().provider != SessionProvider::Claude { return Err(OUTSIDE.into()); }
            let base = source.path().with_extension("");
            let path = safe_join(&base, rel_path)?;
            if !path.starts_with(source.root()) { return Err(OUTSIDE.into()); }
            String::from_utf8(read_limited(&path)?).map_err(|_| NOT_TEXT)?
        }
        ContentRef::Sqlite { .. } => return Err("数据库引用须使用数据库内容读取入口".into()),
    };
    if text.len() as u64 > MAX_TEXT_BYTES { return Err("内容超过32MiB".into()); }
    Ok(text)
}

fn validate_opencode_part(source: &ValidatedSource, path: &Path, part: &Value) -> Result<(), String> {
    if source.identity().provider != SessionProvider::Opencode { return Err(OUTSIDE.into()); }
    let message = part.get("messageID").and_then(Value::as_str).filter(|id| !id.is_empty()).ok_or(OUTSIDE)?;
    if message.contains(['/', '\\']) || message == "." || message == ".." { return Err(OUTSIDE.into()); }
    let relative = path.strip_prefix(source.root()).map_err(|_| OUTSIDE)?;
    if relative.parent() != Some(Path::new("part").join(message).as_path()) { return Err(OUTSIDE.into()); }
    let session = &source.identity().session_id.0;
    let message_path = safe_join(source.root(), &format!("message/{session}/{message}.json"))?;
    let info: Value = serde_json::from_slice(&read_limited(&message_path)?).map_err(|_| STALE)?;
    if info.get("sessionID").and_then(Value::as_str) != Some(session.as_str()) { return Err(OUTSIDE.into()); }
    Ok(())
}

pub(crate) fn safe_join(base: &Path, relative: &str) -> Result<PathBuf, String> {
    let relative = Path::new(relative);
    if relative.as_os_str().is_empty() || relative.components().any(|c| !matches!(c, Component::Normal(_))) { return Err(OUTSIDE.into()); }
    let path = checked_path(&base.join(relative))?;
    let base = checked_path(base)?;
    if !path.starts_with(base) { return Err(OUTSIDE.into()); }
    Ok(path)
}
fn checked_path(path: &Path) -> Result<PathBuf, String> {
    let mut current = PathBuf::new();
    for c in path.components() {
        current.push(c.as_os_str());
        if std::fs::symlink_metadata(&current).map_err(|_| STALE)?.file_type().is_symlink() { return Err(OUTSIDE.into()); }
    }
    path.canonicalize().map_err(|_| STALE.into())
}
pub(crate) fn read_limited(path: &Path) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    File::open(path).map_err(|_| STALE)?.take(MAX_TEXT_BYTES + 1).read_to_end(&mut bytes).map_err(|_| STALE)?;
    if bytes.len() as u64 > MAX_TEXT_BYTES { return Err("内容超过32MiB".into()); }
    Ok(bytes)
}
fn read_jsonl_line(path: &Path, offset: u64, len: u32) -> Result<Vec<u8>, String> {
    if len == 0 || u64::from(len) > MAX_TEXT_BYTES { return Err(STALE.into()); }
    let end = offset.checked_add(u64::from(len)).ok_or(STALE)?;
    let start = offset.saturating_sub(1);
    let mut reader: Box<dyn Read> = if path.extension().is_some_and(|s| s == "zstd") {
        Box::new(zstd::stream::read::Decoder::new(File::open(path).map_err(|_| STALE)?).map_err(|_| STALE)?)
    } else { Box::new(super::rollout::open_reader(path).map_err(|_| STALE)?) };
    if std::io::copy(&mut reader.by_ref().take(start), &mut std::io::sink()).map_err(|_| STALE)? != start { return Err(STALE.into()); }
    let mut buf = Vec::new();
    reader.take(end - start + 1).read_to_end(&mut buf).map_err(|_| STALE)?;
    let index = (offset - start) as usize;
    let wanted = index + len as usize;
    if buf.len() < wanted || (index > 0 && buf.first() != Some(&b'\n')) { return Err(STALE.into()); }
    let mut line = &buf[index..wanted];
    if line.last() == Some(&b'\n') { line = &line[..line.len()-1]; }
    else if buf.len() > wanted && !matches!(buf[wanted], b'\n' | b'\r') { return Err(STALE.into()); }
    if line.last() == Some(&b'\r') { line = &line[..line.len()-1]; }
    if line.is_empty() || line.contains(&b'\n') { return Err(STALE.into()); }
    Ok(line.to_vec())
}

pub(crate) fn extract_text(value: &Value, pointer: &str) -> Result<String, String> {
    let target = if pointer.is_empty() {
        value
    } else {
        value.pointer(pointer).ok_or_else(|| STALE.to_string())?
    };
    match target {
        Value::String(text) => Ok(text.clone()),
        Value::Array(items) => {
            let texts: Vec<&str> = items
                .iter()
                .filter_map(|item| match item {
                    Value::String(text) => Some(text.as_str()),
                    Value::Object(map) => map.get("text").and_then(Value::as_str),
                    _ => None,
                })
                .collect();
            if texts.is_empty() {
                pretty_json(target)
            } else {
                Ok(texts.join("\n"))
            }
        }
        Value::Object(_) => pretty_json(target),
        _ => Err(NOT_TEXT.to_string()),
    }
}

fn pretty_json(value: &Value) -> Result<String, String> {
    serde_json::to_string_pretty(value).map_err(|_| NOT_TEXT.to_string())
}

