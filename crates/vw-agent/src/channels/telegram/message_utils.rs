//! Telegram 消息处理工具模块
//!
//! 本模块提供 Telegram 频道消息处理的核心工具函数，主要功能包括：
//! - 消息分割：将超长消息按 Telegram API 限制拆分为多个片段
//! - 反应表情：为消息添加随机确认反应表情
//! - 请求构建：构造 Telegram 反应 API 的 JSON 请求体
//!
//! # 设计考量
//!
//! - **智能分割**：优先在换行符或空格处分割，保持消息可读性
//! - **长度限制**：严格遵守 Telegram 4096 字符的消息长度上限
//! - **均匀随机**：使用拒绝采样算法确保反应表情的均匀分布

/// Telegram 单条消息的最大字符长度
///
/// 根据 Telegram Bot API 文档，单条消息最多支持 4096 个 UTF-8 字符。
/// 超过此限制的消息需要分割为多条发送。
pub(super) const TELEGRAM_MAX_MESSAGE_LENGTH: usize = 4096;

/// 消息续接的额外开销预留长度
///
/// 当消息需要分割时，每个片段（除最后一个）需要预留额外空间用于：
/// - 消息编号标注（如 "[1/3]"）
/// - 格式化标记
/// - 其他元数据
const TELEGRAM_CONTINUATION_OVERHEAD: usize = 30;

/// 可用的确认反应表情列表
///
/// 这些表情用于在收到用户消息后发送确认反馈。
/// 系统会从中随机选择一个表情作为反应，增加交互的多样性和趣味性。
pub(super) const TELEGRAM_ACK_REACTIONS: &[&str] = &["⚡️", "👌", "👀", "🔥", "👍"];

/// 将消息分割为符合 Telegram 长度限制的多个片段
///
/// 该函数实现了智能消息分割算法，优先在自然边界（换行符、空格）处分割，
/// 以保持消息的可读性和语义完整性。
///
/// # 参数
///
/// - `message`: 待分割的原始消息文本
///
/// # 返回值
///
/// 返回一个字符串向量，每个元素都是一个符合长度限制的消息片段。
/// 如果原始消息未超过长度限制，则返回单元素向量。
///
/// # 分割策略
///
/// 1. 如果消息长度 ≤ 4096 字符，直接返回原消息
/// 2. 对于超长消息，尝试按以下优先级寻找分割点：
///    - 换行符（`\n`）：保持段落完整性
///    - 空格（` `）：保持单词完整性
///    - 硬分割：当找不到自然边界时，在限制处强制分割
/// 3. 每个非最后片段预留 30 字符的续接开销
///
/// # 示例
///
/// ```ignore
/// let long_msg = "很长的消息内容..."; // 假设超过 4096 字符
/// let chunks = split_message_for_telegram(long_msg);
/// for chunk in chunks {
///     assert!(chunk.chars().count() <= 4096);
/// }
/// ```
pub(super) fn split_message_for_telegram(message: &str) -> Vec<String> {
    // 短消息直接返回，无需分割
    if message.chars().count() <= TELEGRAM_MAX_MESSAGE_LENGTH {
        return vec![message.to_string()];
    }

    let mut chunks = Vec::new();
    let mut remaining = message;
    // 计算实际可用长度：最大长度减去续接开销
    let chunk_limit = TELEGRAM_MAX_MESSAGE_LENGTH - TELEGRAM_CONTINUATION_OVERHEAD;

    while !remaining.is_empty() {
        // 剩余部分可直接作为最后一段
        if remaining.chars().count() <= TELEGRAM_MAX_MESSAGE_LENGTH {
            chunks.push(remaining.to_string());
            break;
        }

        // 计算硬分割位置（字节索引）
        // char_indices() 用于正确处理多字节 UTF-8 字符
        let hard_split =
            remaining.char_indices().nth(chunk_limit).map_or(remaining.len(), |(idx, _)| idx);

        let search_area = &remaining[..hard_split];

        // 智能寻找最佳分割点
        let chunk_end = if let Some(pos) = search_area.rfind('\n') {
            // 只有当换行符前仍有足够内容时才在此分割
            // 避免产生过短的片段
            if search_area[..pos].chars().count() >= chunk_limit / 2 {
                pos + 1 // 包含换行符在当前片段中
            } else if let Some(pos) = search_area.rfind(' ') {
                // 换行符太靠前，改为在空格处分割
                pos + 1
            } else {
                hard_split
            }
        } else if let Some(pos) = search_area.rfind(' ') {
            // 其次在空格处分割（保持单词完整）
            pos + 1 // 空格归属当前片段，下一段从新词开始
        } else {
            // 没有找到自然边界，执行硬分割
            hard_split
        };

        // 提取当前片段并添加到结果中
        chunks.push(remaining[..chunk_end].to_string());
        // 更新剩余待处理内容
        remaining = &remaining[chunk_end..];
    }

    chunks
}

/// 使用拒绝采样算法生成均匀分布的随机索引
///
/// 该函数实现了高质量的均匀随机数生成，避免了简单取模算法在小范围时的偏差问题。
/// 通过拒绝采样确保每个索引被选中的概率严格相等。
///
/// # 参数
///
/// - `len`: 索引范围的上界（有效索引为 0..len）
///
/// # 返回值
///
/// 返回 [0, len) 范围内的均匀分布随机索引
///
/// # 算法原理
///
/// 1. 计算拒绝阈值：`reject_threshold = (u64::MAX / len) * len`
/// 2. 生成随机值，如果 >= 阈值则拒绝并重试
/// 3. 对接受的值取模得到最终索引
///
/// 这样可以消除取模操作在范围不整除时引入的微小偏差。
fn pick_uniform_index(len: usize) -> usize {
    debug_assert!(len > 0);
    let upper = len as u64;
    // 计算拒绝阈值：大于等于此值的随机数会被拒绝
    // 这确保了取模后的分布是均匀的
    let reject_threshold = (u64::MAX / upper) * upper;

    loop {
        let value = rand::random::<u64>();
        // 只接受低于阈值的随机值，避免分布偏差
        if value < reject_threshold {
            #[allow(clippy::cast_possible_truncation)]
            return (value % upper) as usize;
        }
    }
}

/// 从预定义列表中随机选择一个确认反应表情
///
/// 该函数使用均匀分布的随机算法从 `TELEGRAM_ACK_REACTIONS` 中选择一个表情，
/// 用于对用户消息进行确认反馈。
///
/// # 返回值
///
/// 返回一个静态生命周期的字符串切片，表示选中的表情符号
///
/// # 示例
///
/// ```ignore
/// let emoji = random_telegram_ack_reaction();
/// // emoji 可能是 "⚡️"、"👌"、"👀"、"🔥" 或 "👍" 中的任意一个
/// ```
pub(super) fn random_telegram_ack_reaction() -> &'static str {
    TELEGRAM_ACK_REACTIONS[pick_uniform_index(TELEGRAM_ACK_REACTIONS.len())]
}

/// 构建 Telegram 消息反应 API 请求体
///
/// 该函数构造一个 JSON 对象，用于调用 Telegram Bot API 的 setMessageReaction 方法。
/// 通过此请求可以为指定消息添加表情反应。
///
/// # 参数
///
/// - `chat_id`: 目标聊天的唯一标识符（可以是用户 ID、群组 ID 或频道用户名）
/// - `message_id`: 目标消息在聊天中的唯一序号
/// - `emoji`: 要添加的表情符号（必须来自 Telegram 支持的表情集）
///
/// # 返回值
///
/// 返回一个 `serde_json::Value`，包含完整的 API 请求体，可直接用于 HTTP 请求
///
/// # 请求体结构
///
/// ```json
/// {
///     "chat_id": "聊天标识",
///     "message_id": 消息序号,
///     "reaction": [{
///         "type": "emoji",
///         "emoji": "表情符号"
///     }]
/// }
/// ```
///
/// # 示例
///
/// ```ignore
/// let request = build_telegram_ack_reaction_request("123456789", 42, "👍");
/// // 可直接用于 Telegram API 调用
/// ```
pub(super) fn build_telegram_ack_reaction_request(
    chat_id: &str,
    message_id: i64,
    emoji: &str,
) -> serde_json::Value {
    serde_json::json!({
        "chat_id": chat_id,
        "message_id": message_id,
        "reaction": [{
            "type": "emoji",
            "emoji": emoji
        }]
    })
}
