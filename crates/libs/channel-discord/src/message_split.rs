//! # Discord 消息分割模块
//!
//! 本模块提供 Discord 消息长度限制的处理功能。
//!
//! ## 功能概述
//!
//! Discord 对单条消息有严格的字符长度限制（2000字符），超出限制的消息会被拒绝
//! 并返回 `50035 Invalid Form Body` 错误。本模块实现了智能消息分割策略：
//!
//! - **边界感知分割**：优先在自然边界（换行符、空格）处分割消息
//! - **最小化分割**：当消息未超出限制时直接返回，避免不必要的处理
//! - **UTF-8 安全**：正确处理多字节 Unicode 字符，按字符计数而非字节
//!
//! ## 使用场景
//!
//! 该模块主要用于 Discord 通道发送长消息前的预处理，确保消息能够被
//! Discord API 正常接收。

/// Discord 普通消息的最大字符长度限制。
///
/// Discord API 对单条消息有 2000 字符的硬性限制。
/// 超出此长度的消息将收到 `50035 Invalid Form Body` 错误响应。
///
/// # 技术说明
///
/// - 此限制基于 Unicode 字符数（码点），而非字节数
/// - Discord 还存在其他限制（如 embed 字段限制），此常量仅针对普通文本消息
pub(super) const DISCORD_MAX_MESSAGE_LENGTH: usize = 2000;

/// 将消息分割为符合 Discord 长度限制的多个块。
///
/// 该函数实现了智能分割策略，尽可能在自然的文本边界处进行分割，
/// 以保持消息的可读性和语义完整性。
///
/// # 分割策略
///
/// 1. **无需分割**：若消息长度 ≤ 2000 字符，直接返回原消息
/// 2. **换行符优先**：在前 2000 字符范围内寻找最后一个换行符 `\n`
/// 3. **空格次选**：若无合适的换行符，尝试在空格处分割
/// 4. **强制分割**：若无任何自然边界，在限制处强制截断
///
/// # 参数
///
/// * `message` - 待分割的原始消息字符串
///
/// # 返回值
///
/// 返回一个 `Vec<String>`，包含分割后的所有消息块。
/// 每个块的长度都不会超过 [`DISCORD_MAX_MESSAGE_LENGTH`]。
///
/// # 示例
///
/// ```ignore
/// use channel_discord::message_split::split_message_for_discord;
///
/// // 短消息不分割
/// let short = "Hello, Discord!";
/// assert_eq!(split_message_for_discord(short), vec!["Hello, Discord!"]);
///
/// // 长消息被分割为多个块
/// let long = "word ".repeat(500);  // 约 2500 字符
/// let chunks = split_message_for_discord(&long);
/// assert!(chunks.len() > 1);
/// for chunk in &chunks {
///     assert!(chunk.chars().count() <= 2000);
/// }
/// ```
///
/// # 边界条件处理
///
/// - 空字符串返回空向量
/// - 恰好 2000 字符的消息不分割
/// - 多字节 UTF-8 字符（如中文、emoji）被正确处理
pub(super) fn split_message_for_discord(message: &str) -> Vec<String> {
    // 快速路径：消息未超出限制，无需分割
    if message.chars().count() <= DISCORD_MAX_MESSAGE_LENGTH {
        return vec![message.to_string()];
    }

    let mut chunks = Vec::new();
    let mut remaining = message;

    // 循环处理剩余文本，直到全部分割完成
    while !remaining.is_empty() {
        // 找到第 2000 个字符的字节偏移位置
        // 这是 Unicode 安全的：char_indices() 按 Unicode 码点迭代
        // 如果剩余字符数不足 2000，则返回剩余文本的总长度
        let hard_split = remaining
            .char_indices()
            .nth(DISCORD_MAX_MESSAGE_LENGTH)
            .map_or(remaining.len(), |(idx, _)| idx);

        let chunk_end = if hard_split == remaining.len() {
            // 剩余文本不足 2000 字符，直接使用剩余全部
            hard_split
        } else {
            // 在限制范围内寻找最佳分割点
            let search_area = &remaining[..hard_split];

            // 策略 1：优先在换行符处分割（保持段落完整性）
            if let Some(pos) = search_area.rfind('\n') {
                // 确保分割点不会太靠后（至少保留一半长度）
                // 避免"为了一个换行符而牺牲大量空间"的情况
                if search_area[..pos].chars().count() >= DISCORD_MAX_MESSAGE_LENGTH / 2 {
                    // 包含换行符在当前块中
                    pos + 1
                } else {
                    // 换行符太靠后，改用空格作为备选分割点
                    search_area.rfind(' ').map_or(hard_split, |space| space + 1)
                }
            } else if let Some(pos) = search_area.rfind(' ') {
                // 策略 2：在空格处分割（保持单词完整性）
                pos + 1
            } else {
                // 策略 3：无自然边界，在限制处强制分割
                // 这种情况在连续无空格的长文本（如 URL 或 Base64）中出现
                hard_split
            }
        };

        // 将当前块添加到结果中
        chunks.push(remaining[..chunk_end].to_string());
        // 更新剩余待处理的文本
        remaining = &remaining[chunk_end..];
    }

    chunks
}

#[cfg(test)]
#[path = "message_split_tests.rs"]
mod message_split_tests;
