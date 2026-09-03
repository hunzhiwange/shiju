use once_cell::sync::Lazy;
use std::borrow::Cow;

use crate::time::now_ms;

const TASK_SESSION_SOURCE_OPEN: &str = "<task_session_source>\n";
const TASK_SESSION_SOURCE_ID_PREFIX: &str = "source_session_id: ";
pub const TASK_SESSION_SOURCE_LABEL: &str = "由 ChatGPT 从另一项任务发起";
const TASK_SESSION_SOURCE_CLOSE: &str = "</task_session_source>\n";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskSessionSource<'a> {
    pub source_session_id: &'a str,
    pub content: &'a str,
}

/// 为跨任务用户消息添加可持久化的内部来源块。
pub fn encode_task_session_source_marker(source_session_id: &str, content: &str) -> Option<String> {
    let source_session_id = source_session_id.trim();
    if source_session_id.is_empty() || source_session_id.lines().count() != 1 {
        return None;
    }

    Some(format!(
        "{TASK_SESSION_SOURCE_OPEN}{TASK_SESSION_SOURCE_ID_PREFIX}{source_session_id}\n{TASK_SESSION_SOURCE_LABEL}\n{TASK_SESSION_SOURCE_CLOSE}{content}"
    ))
}

/// 严格解析位于消息开头的完整跨任务来源块。
pub fn parse_task_session_source_marker(raw: &str) -> Option<TaskSessionSource<'_>> {
    let after_open = raw.strip_prefix(TASK_SESSION_SOURCE_OPEN)?;
    let after_id_prefix = after_open.strip_prefix(TASK_SESSION_SOURCE_ID_PREFIX)?;
    let (source_session_id, rest) = after_id_prefix.split_once('\n')?;
    if source_session_id.is_empty() || source_session_id.trim() != source_session_id {
        return None;
    }
    let after_description = rest
        .strip_prefix(TASK_SESSION_SOURCE_LABEL)?
        .strip_prefix('\n')?;
    let content = after_description.strip_prefix(TASK_SESSION_SOURCE_CLOSE)?;

    Some(TaskSessionSource { source_session_id, content })
}

/// 展示或复制消息前剥离合法来源块；畸形内容保持原样。
pub fn strip_task_session_source_marker(raw: &str) -> Cow<'_, str> {
    parse_task_session_source_marker(raw)
        .map_or_else(|| Cow::Borrowed(raw), |source| Cow::Borrowed(source.content))
}

/// 生成短随机 slug，失败时回退为当前时间戳。
pub fn create_slug() -> String {
    let mut bytes = [0u8; 8];
    slug_from_random_result(getrandom::getrandom(&mut bytes).map(|_| bytes).map_err(|_| ()))
}

fn slug_from_random_result(random: Result<[u8; 8], ()>) -> String {
    match random {
        Ok(bytes) => slug_from_bytes(bytes),
        Err(()) => format!("{:x}", now_ms()),
    }
}

fn slug_from_bytes(bytes: [u8; 8]) -> String {
    const CHARS: &[u8; 62] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    let mut out = String::with_capacity(bytes.len());
    for b in bytes {
        out.push(CHARS[(b as usize) % 62] as char);
    }
    out
}

/// 判断标题是否仍是系统生成的默认标题。
pub fn is_default_title(title: &str) -> bool {
    static RE: Lazy<regex::Regex> = Lazy::new(|| {
        regex::Regex::new(
            r"^(New session - |Child session - )\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$",
        )
        .unwrap()
    });
    RE.is_match(title)
}

#[cfg(test)]
#[path = "session_utils_tests.rs"]
mod session_utils_tests;
