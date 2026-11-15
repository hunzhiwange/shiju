//! doctor 子命令共享的小工具。
//!
//! 这里仅放置多个诊断模块共同需要的轻量辅助函数，避免在具体检查模块中重复
//! 字符串截断与时间解析逻辑。

use chrono::{DateTime, Utc};

/// 截断用于终端展示的字符串。
///
/// 参数：
/// - `input`：原始文本。
/// - `max_chars`：最多保留的 Unicode scalar 数量。
///
/// 返回值：
/// 如果输入超过限制，返回带省略号的预览文本；否则返回原文本副本。
pub(super) fn truncate_for_display(input: &str, max_chars: usize) -> String {
    let mut chars = input.chars();
    let preview: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() { format!("{preview}…") } else { preview }
}

/// 解析 RFC3339 时间戳并转换为 UTC。
///
/// 参数：
/// - `raw`：待解析的时间戳文本。
///
/// 返回值：
/// 解析成功时返回 UTC 时间；格式无效时返回 `None`，由调用方决定诊断等级。
pub(super) fn parse_rfc3339(raw: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw).ok().map(|dt| dt.with_timezone(&Utc))
}
