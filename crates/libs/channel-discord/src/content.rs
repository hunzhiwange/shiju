//! Discord 消息内容处理工具模块
//!
//! 本模块提供 Discord 通道消息内容的标准化处理功能，包括：
//! - 发送者 ID 列表的规范化与去重
//! - 机器人提及标签的生成与检测
//! - 入站消息内容的预处理与清理
//!
//! 所有函数均为 `pub(super)` 可见性，仅在 `discord` 模块内部使用。

/// 规范化群组回复允许的发送者 ID 列表
///
/// 对原始发送者 ID 列表执行以下处理：
/// 1. 去除每个条目前后的空白字符
/// 2. 过滤掉空字符串
/// 3. 按字母顺序排序
/// 4. 移除重复项
///
/// # 参数
///
/// * `sender_ids` - 原始发送者 ID 列表，可能包含空白、重复项或无效条目
///
/// # 返回值
///
/// 返回经过规范化处理的发送者 ID 列表，保证：
/// - 无前后空白
/// - 无空字符串
/// - 按字母顺序排列
/// - 无重复项
///
/// # 示例
///
/// ```ignore
/// use super::content::normalize_group_reply_allowed_sender_ids;
///
/// let ids = vec!["  alice  ".to_string(), "bob".to_string(), "alice".to_string(), "".to_string()];
/// let normalized = normalize_group_reply_allowed_sender_ids(ids);
/// assert_eq!(normalized, vec!["alice", "bob"]);
/// ```
pub(super) fn normalize_group_reply_allowed_sender_ids(sender_ids: Vec<String>) -> Vec<String> {
    // 步骤 1-2: 遍历列表，去除空白并过滤空条目
    let mut normalized = sender_ids
        .into_iter()
        .map(|entry| entry.trim().to_string())
        .filter(|entry| !entry.is_empty())
        .collect::<Vec<_>>();

    // 步骤 3: 按字母顺序排序，为去重做准备
    normalized.sort();

    // 步骤 4: 移除相邻的重复项（需先排序）
    normalized.dedup();

    normalized
}

/// 生成机器人的提及标签格式
///
/// Discord 支持两种用户提及格式：
/// - `<@用户ID>` - 标准提及格式
/// - `<@!用户ID>` - 带昵称标记的提及格式
///
/// # 参数
///
/// * `bot_user_id` - Discord 机器人的用户 ID（数字字符串）
///
/// # 返回值
///
/// 返回包含两种提及标签格式的数组：`[标准格式, 昵称格式]`
///
/// # 示例
///
/// ```ignore
/// use super::content::mention_tags;
///
/// let tags = mention_tags("123456789");
/// assert_eq!(tags[0], "<@123456789>");
/// assert_eq!(tags[1], "<@!123456789>");
/// ```
pub(super) fn mention_tags(bot_user_id: &str) -> [String; 2] {
    [format!("<@{bot_user_id}>"), format!("<@!{bot_user_id}>")]
}

/// 检查消息内容是否包含机器人提及
///
/// 检测消息中是否包含对当前机器人的提及（@提及），支持标准格式和昵称格式。
///
/// # 参数
///
/// * `content` - 待检查的消息文本内容
/// * `bot_user_id` - Discord 机器人的用户 ID
///
/// # 返回值
///
/// - `true` - 消息中包含对机器人的提及
/// - `false` - 消息中不包含对机器人的提及
///
/// # 示例
///
/// ```ignore
/// use super::content::contains_bot_mention;
///
/// let content = "你好 <@123456789>，请帮我处理这个任务";
/// assert!(contains_bot_mention(content, "123456789"));
///
/// let content2 = "这是一条普通消息";
/// assert!(!contains_bot_mention(content2, "123456789"));
/// ```
pub(super) fn contains_bot_mention(content: &str, bot_user_id: &str) -> bool {
    // 获取两种提及标签格式
    let tags = mention_tags(bot_user_id);

    // 检查内容是否包含任一提及格式
    content.contains(&tags[0]) || content.contains(&tags[1])
}

/// 规范化入站消息内容
///
/// 对接收到的 Discord 消息内容进行预处理，包括：
/// - 验证消息非空
/// - 根据配置检查是否需要机器人提及
/// - 移除消息中的机器人提及标签
/// - 清理首尾空白
///
/// # 参数
///
/// * `content` - 原始消息文本内容
/// * `require_mention` - 是否要求消息必须包含机器人提及才能处理
/// * `bot_user_id` - Discord 机器人的用户 ID
///
/// # 返回值
///
/// - `Some(String)` - 规范化后的消息内容
/// - `None` - 消息不符合处理条件，可能原因：
///   - 原始内容为空
///   - `require_mention` 为 true 但消息未提及机器人
///   - 移除提及标签后内容变为空白
///
/// # 示例
///
/// ```ignore
/// use super::content::normalize_incoming_content;
///
/// // 需要提及的情况
/// let content = "<@123456789> 你好，请帮我分析一下";
/// let result = normalize_incoming_content(content, true, "123456789");
/// assert_eq!(result, Some("你好，请帮我分析一下".to_string()));
///
/// // 未提及机器人，返回 None
/// let content2 = "这是一条普通消息";
/// let result2 = normalize_incoming_content(content2, true, "123456789");
/// assert_eq!(result2, None);
///
/// // 不需要提及的情况
/// let content3 = "直接处理这条消息";
/// let result3 = normalize_incoming_content(content3, false, "123456789");
/// assert_eq!(result3, Some("直接处理这条消息".to_string()));
/// ```
pub(super) fn normalize_incoming_content(
    content: &str,
    require_mention: bool,
    bot_user_id: &str,
) -> Option<String> {
    // 空消息直接返回 None
    if content.is_empty() {
        return None;
    }

    // 如果要求提及但消息未提及机器人，返回 None
    if require_mention && !contains_bot_mention(content, bot_user_id) {
        return None;
    }

    // 创建可修改的副本
    let mut normalized = content.to_string();

    // 如果要求提及，移除消息中的机器人提及标签
    // 将提及替换为空格而非直接删除，以避免词语粘连
    if require_mention {
        for tag in mention_tags(bot_user_id) {
            normalized = normalized.replace(&tag, " ");
        }
    }

    // 清理首尾空白
    let normalized = normalized.trim().to_string();

    // 如果处理后内容为空，返回 None
    if normalized.is_empty() {
        return None;
    }

    Some(normalized)
}

#[cfg(test)]
#[path = "content_tests.rs"]
mod content_tests;
