//! CLI 文本截断工具模块
//!
//! 本模块提供用于 CLI 输出的文本截断功能。在终端环境中显示长文本时，
//! 需要限制字符数量以保持输出的可读性和布局一致性。
//!
//! # 设计原则
//!
//! - **Unicode 安全**: 正确处理多字节 UTF-8 字符，按字符而非字节截断
//! - **视觉提示**: 截断时添加省略号（`…`）提示用户内容被截断
//! - **零分配优化**: 仅在必要时分配输出字符串

/// 将字符串截断到指定的最大字符数
///
/// 此函数按字符（而非字节）截断字符串，确保正确处理 Unicode 字符。
/// 当字符串超过最大长度时，会在末尾添加省略号（`…`）。
///
/// # 参数
///
/// * `s` - 待截断的输入字符串
/// * `max_chars` - 最大字符数（不包括省略号）
///
/// # 返回值
///
/// 返回截断后的字符串：
/// - 如果 `max_chars` 为 0，返回空字符串
/// - 如果输入字符串长度不超过 `max_chars`，返回原字符串的克隆
/// - 如果输入字符串超过 `max_chars`，返回前 `max_chars` 个字符加省略号
///
/// # 示例
///
/// ```ignore
/// use crate::truncate::truncate_chars_cli;
///
/// // 正常截断
/// let result = truncate_chars_cli("Hello, World!", 5);
/// assert_eq!(result, "Hello…");
///
/// // 字符串短于限制，不截断
/// let result = truncate_chars_cli("Hi", 10);
/// assert_eq!(result, "Hi");
///
/// // 最大字符数为 0，返回空字符串
/// let result = truncate_chars_cli("Any text", 0);
/// assert_eq!(result, "");
///
/// // Unicode 字符正确处理
/// let result = truncate_chars_cli("你好世界", 2);
/// assert_eq!(result, "你好…");
/// ```
pub(crate) fn truncate_chars_cli(s: &str, max_chars: usize) -> String {
    // 边界情况：最大字符数为 0 时直接返回空字符串
    if max_chars == 0 {
        return String::new();
    }

    let mut out = String::new();

    // 逐字符遍历，确保正确处理 Unicode
    for (idx, ch) in s.chars().enumerate() {
        // 达到最大字符数时，添加省略号并返回
        if idx >= max_chars {
            out.push('…');
            return out;
        }
        out.push(ch);
    }

    // 字符串未超过限制，原样返回
    out
}
