//! Telegram 频道核心功能模块
//!
//! 本模块提供 TelegramChannel 的核心辅助方法，用于支持 Telegram Bot API 的集成。
//! 主要功能包括：
//! - HTTP 客户端构建（支持代理配置）
//! - API URL 构建
//! - 用户权限验证
//! - 错误信息净化处理
//!
//! 这些方法作为内部辅助函数，供 TelegramChannel 的其他模块使用。

use super::TelegramChannel;

impl TelegramChannel {
    /// 构建并返回配置了代理的 HTTP 客户端
    ///
    /// 使用全局运行时代理配置创建 HTTP 客户端，适用于 Telegram API 调用。
    ///
    /// # 返回值
    ///
    /// 返回一个配置了代理的 `reqwest::Client` 实例
    pub(super) fn http_client(&self) -> reqwest::Client {
        crate::channels::http::proxied_client("channel.telegram")
    }

    /// 净化 Telegram API 错误信息
    ///
    /// 移除或屏蔽敏感信息，确保错误信息可安全记录和展示。
    ///
    /// # 参数
    ///
    /// * `input` - 原始错误信息字符串
    ///
    /// # 返回值
    ///
    /// 返回净化后的错误信息字符串
    pub(super) fn sanitize_telegram_error(input: &str) -> String {
        let sanitized = crate::app::agent::providers::sanitize_api_error(input);
        static BOT_TOKEN_RE: std::sync::LazyLock<regex::Regex> =
            std::sync::LazyLock::new(|| regex::Regex::new(r"bot[^/\s]+").unwrap());
        BOT_TOKEN_RE.replace_all(&sanitized, "bot[redacted]").into_owned()
    }

    /// 构建 Telegram Bot API 请求 URL
    ///
    /// 根据配置的 API 基础地址和 Bot Token，生成完整的 API 方法 URL。
    ///
    /// # 参数
    ///
    /// * `method` - Telegram API 方法名称（如 "sendMessage"、"getUpdates" 等）
    ///
    /// # 返回值
    ///
    /// 返回完整的 API URL 字符串，格式为：`{api_base}/bot{token}/{method}`
    pub(super) fn api_url(&self, method: &str) -> String {
        format!("{}/bot{}/{method}", self.api_base, self.bot_token)
    }

    /// 检查用户是否在允许列表中
    ///
    /// 验证给定的用户列表中是否有任何用户被授权使用此 Bot。
    /// 支持通配符 "*" 表示允许所有用户。
    ///
    /// # 参数
    ///
    /// * `users` - 待检查的用户标识符迭代器（通常是用户 ID 或用户名的字符串引用）
    ///
    /// # 返回值
    ///
    /// - `true` - 至少有一个用户被授权（或允许列表包含通配符 "*"）
    /// - `false` - 所有用户均未被授权
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let channel = TelegramChannel::new(config);
    ///
    /// // 检查单个用户
    /// let allowed = channel.is_any_user_allowed(["user123"].iter().map(|s| *s));
    ///
    /// // 检查多个用户
    /// let users = vec!["user1", "user2"];
    /// let allowed = channel.is_any_user_allowed(users.iter().map(|s| s.as_str()));
    /// ```
    pub(super) fn is_any_user_allowed<'a>(&self, users: impl Iterator<Item = &'a str>) -> bool {
        // 获取允许列表的读锁，如果锁被污染则恢复其内部值
        let allowed = self.allowed_users.read().unwrap_or_else(|e| e.into_inner());

        // 如果允许列表包含通配符 "*"，则允许所有用户
        if (*allowed).iter().any(|u| u == "*") {
            return true;
        }

        // 遍历待检查的用户列表，检查是否有任一用户在允许列表中
        for user in users {
            if allowed.contains(&user.to_string()) {
                return true;
            }
        }

        // 所有用户均未被授权
        false
    }

    /// 处理未授权消息
    ///
    /// 当收到未授权用户发送的消息时调用此方法进行日志记录。
    ///
    /// # 参数
    ///
    /// * `update` - Telegram 更新对象的 JSON 值
    ///
    /// # 说明
    ///
    /// 当前实现仅记录警告日志，不执行其他操作。
    /// 可根据需要扩展为发送回复消息或执行其他安全措施。
    pub(super) async fn handle_unauthorized_message(&self, update: &serde_json::Value) {
        tracing::warn!("Unauthorized message received: {:?}", update);
    }
}
