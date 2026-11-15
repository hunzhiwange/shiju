//! Discord 权限管理模块
//!
//! 本模块提供 Discord 频道的权限验证功能，用于控制哪些用户和发送者可以触发代理响应。
//! 权限验证基于白名单机制，支持通配符匹配。
//!
//! # 主要功能
//!
//! - **用户权限验证**：检查特定用户 ID 是否在允许列表中
//! - **群组发送者验证**：在群组场景中验证发送者是否有权触发响应
//!
//! # 使用示例
//!
//! ```ignore
//! use channel_discord::permissions;
//!
//! // 用户权限验证
//! let allowed_users = vec!["123456789".to_string(), "*".to_string()];
//! let is_allowed = permissions::is_user_allowed(&allowed_users, "123456789");
//! assert!(is_allowed);
//!
//! // 群组发送者验证
//! let allowed_senders = vec!["111222333".to_string()];
//! let can_trigger = permissions::is_group_sender_trigger_enabled(&allowed_senders, "111222333");
//! assert!(can_trigger);
//! ```

/// 检查用户是否在允许列表中
///
/// 此函数用于验证特定用户 ID 是否有权限与代理交互。支持通配符 `"*"` 表示允许所有用户。
///
/// # 参数
///
/// * `allowed_users` - 允许的用户 ID 列表，可以包含通配符 `"*"` 表示允许所有用户
/// * `user_id` - 待验证的用户 ID 字符串
///
/// # 返回值
///
/// 返回 `true` 表示用户被允许，`false` 表示用户不在允许列表中
///
/// # 示例
///
/// ```ignore
/// // 允许所有用户
/// let allowed = vec!["*".to_string()];
/// assert!(is_user_allowed(&allowed, "any_user_id"));
///
/// // 仅允许特定用户
/// let allowed = vec!["123456789".to_string()];
/// assert!(is_user_allowed(&allowed, "123456789"));
/// assert!(!is_user_allowed(&allowed, "987654321"));
/// ```
pub(super) fn is_user_allowed(allowed_users: &[String], user_id: &str) -> bool {
    // 遍历允许列表，如果匹配通配符 "*" 或精确匹配用户 ID，则返回 true
    allowed_users.iter().any(|u| u == "*" || u == user_id)
}

/// 检查群组中的发送者是否启用了触发器
///
/// 此函数用于在群组（如 Discord 服务器频道）场景中验证发送者是否有权触发代理响应。
/// 与 `is_user_allowed` 类似，支持通配符 `"*"` 表示允许所有发送者。
/// 如果发送者 ID 为空或仅包含空白字符，将拒绝触发。
///
/// # 参数
///
/// * `group_reply_allowed_sender_ids` - 允许触发响应的发送者 ID 列表，
///   可以包含通配符 `"*"` 表示允许所有发送者
/// * `sender_id` - 待验证的发送者 ID 字符串
///
/// # 返回值
///
/// 返回 `true` 表示发送者有权触发响应，`false` 表示无权或发送者 ID 无效
///
/// # 示例
///
/// ```ignore
/// // 允许所有发送者
/// let allowed = vec!["*".to_string()];
/// assert!(is_group_sender_trigger_enabled(&allowed, "any_sender"));
///
/// // 仅允许特定发送者
/// let allowed = vec!["111222333".to_string()];
/// assert!(is_group_sender_trigger_enabled(&allowed, "111222333"));
/// assert!(!is_group_sender_trigger_enabled(&allowed, "999888777"));
///
/// // 空发送者 ID 被拒绝
/// assert!(!is_group_sender_trigger_enabled(&allowed, ""));
/// assert!(!is_group_sender_trigger_enabled(&allowed, "   "));
/// ```
pub(super) fn is_group_sender_trigger_enabled(
    group_reply_allowed_sender_ids: &[String],
    sender_id: &str,
) -> bool {
    // 去除发送者 ID 的前后空白字符
    let sender_id = sender_id.trim();

    // 如果发送者 ID 为空，拒绝触发
    if sender_id.is_empty() {
        return false;
    }

    // 遍历允许列表，如果匹配通配符 "*" 或精确匹配发送者 ID，则返回 true
    group_reply_allowed_sender_ids.iter().any(|entry| entry == "*" || entry == sender_id)
}

#[cfg(test)]
#[path = "permissions_tests.rs"]
mod permissions_tests;
