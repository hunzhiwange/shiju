//! Telegram 通道配置模块
//!
//! 本模块提供 Telegram 机器人通道的配置和初始化功能，包括：
//! - 通道实例的创建和配置
//! - 用户身份规范化处理
//! - 数据库配置的加载和持久化
//! - 配对码机制的实现
//! - 机器人用户名的获取和缓存
//!
//! ## 核心功能
//!
//! - **身份规范化**：统一处理 Telegram 用户名格式，去除 `@` 前缀和空白字符
//! - **配对机制**：当未配置允许用户列表时，自动启用配对码进行安全的用户绑定
//! - **配置持久化**：支持将新绑定的用户身份持久化到共享数据库
//! - **流式响应**：支持流式模式下的草稿消息更新

use super::TelegramChannel;
use crate::app::agent::config::{Config, StreamMode};
use crate::app::agent::security::pairing::PairingGuard;
use anyhow::Context;
use parking_lot::Mutex;
use std::sync::{Arc, RwLock};

/// Telegram 绑定命令常量
///
/// 用户通过发送此命令加配对码来完成与机器人的绑定。
/// 例如：`/bind ABC123`
const TELEGRAM_BIND_COMMAND: &str = "/bind";

impl TelegramChannel {
    /// 创建新的 Telegram 通道实例
    ///
    /// # 参数
    ///
    /// - `bot_token`: Telegram Bot API 令牌，由 BotFather 分发
    /// - `allowed_users`: 允许与机器人交互的用户名列表（会被自动规范化）
    /// - `mention_only`: 是否仅响应 @提及 消息（群聊场景）
    ///
    /// # 返回值
    ///
    /// 返回配置好的 `TelegramChannel` 实例
    ///
    /// # 配对模式
    ///
    /// 当 `allowed_users` 为空时，自动启用配对模式：
    /// - 生成一次性配对码并打印到控制台
    /// - 用户需发送 `/bind <code>` 完成绑定
    /// - 绑定后的用户身份会被持久化到配置文件
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let channel = TelegramChannel::new(
    ///     "123456:ABC-DEF".to_string(),
    ///     vec!["alice".to_string(), "@bob".to_string()],
    ///     false,
    /// );
    /// ```
    pub fn new(bot_token: String, allowed_users: Vec<String>, mention_only: bool) -> Self {
        // 规范化允许用户列表（去除 @ 前缀和空白）
        let normalized_allowed = Self::normalize_allowed_users(allowed_users);

        Self {
            bot_token,
            allowed_users: Arc::new(RwLock::new(normalized_allowed)),
            pairing: None,
            client: reqwest::Client::new(),
            stream_mode: StreamMode::Off,
            draft_update_interval_ms: 1000,
            last_draft_edit: Mutex::new(std::collections::HashMap::new()),
            typing_handle: Mutex::new(None),
            mention_only,
            group_reply_allowed_sender_ids: Vec::new(),
            bot_username: Mutex::new(None),
            api_base: "https://api.telegram.org".to_string(),
            workspace_dir: None,
        }
    }

    /// 配置工作空间目录
    ///
    /// 设置用于保存下载附件的目录路径。
    ///
    /// # 参数
    ///
    /// - `dir`: 工作空间目录的路径
    ///
    /// # 返回值
    ///
    /// 返回修改后的 `Self`，支持链式调用
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let channel = TelegramChannel::new(token, users, false)
    ///     .with_workspace_dir(PathBuf::from("/var/vibewindow/workspace"));
    /// ```
    pub fn with_workspace_dir(mut self, dir: std::path::PathBuf) -> Self {
        self.workspace_dir = Some(dir);
        self
    }

    /// 配置流式响应模式
    ///
    /// 启用并配置流式模式下的草稿消息更新功能。
    /// 在流式模式下，机器人会定期更新消息内容以展示生成进度。
    ///
    /// # 参数
    ///
    /// - `stream_mode`: 流式模式配置（Off/Progressive/Complete）
    /// - `draft_update_interval_ms`: 草稿更新的间隔时间（毫秒）
    ///
    /// # 返回值
    ///
    /// 返回修改后的 `Self`，支持链式调用
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let channel = TelegramChannel::new(token, users, false)
    ///     .with_streaming(StreamMode::Progressive, 500);
    /// ```
    pub fn with_streaming(
        mut self,
        stream_mode: StreamMode,
        draft_update_interval_ms: u64,
    ) -> Self {
        self.stream_mode = stream_mode;
        self.draft_update_interval_ms = draft_update_interval_ms;
        self
    }

    /// 配置群聊中绕过 @提及 限制的发送者
    ///
    /// 在 `mention_only` 模式下，通常只有 @提及 机器人的消息才会触发响应。
    /// 此方法允许配置特定发送者，他们的消息即使没有 @提及 也会触发回复。
    ///
    /// # 参数
    ///
    /// - `sender_ids`: 允许的发送者 ID 列表，支持使用 "*" 作为通配符
    ///
    /// # 返回值
    ///
    /// 返回修改后的 `Self`，支持链式调用
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let channel = TelegramChannel::new(token, users, true)
    ///     .with_group_reply_allowed_senders(vec!["123456789".to_string(), "*".to_string()]);
    /// ```
    pub fn with_group_reply_allowed_senders(mut self, sender_ids: Vec<String>) -> Self {
        self.group_reply_allowed_sender_ids =
            Self::normalize_group_reply_allowed_sender_ids(sender_ids);
        self
    }

    /// 覆盖 Telegram Bot API 基础 URL
    ///
    /// 默认使用官方 API 地址，此方法允许指定自定义 API 端点。
    /// 适用于本地 Bot API 服务器或测试环境。
    ///
    /// # 参数
    ///
    /// - `api_base`: 自定义 API 基础 URL（如 "http://localhost:8081"）
    ///
    /// # 返回值
    ///
    /// 返回修改后的 `Self`，支持链式调用
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let channel = TelegramChannel::new(token, users, false)
    ///     .with_api_base("http://localhost:8081".to_string());
    /// ```
    pub fn with_api_base(mut self, api_base: String) -> Self {
        self.api_base = api_base.trim_end_matches('/').to_string();
        self
    }

    /// 规范化用户身份标识
    ///
    /// 处理 Telegram 用户名格式，去除 `@` 前缀和首尾空白。
    ///
    /// # 参数
    ///
    /// - `value`: 原始身份字符串（可能包含 @ 前缀或空白）
    ///
    /// # 返回值
    ///
    /// 返回规范化后的身份字符串
    ///
    /// # 示例
    ///
    /// ```ignore
    /// assert_eq!(TelegramChannel::normalize_identity("@alice"), "alice");
    /// assert_eq!(TelegramChannel::normalize_identity("  bob  "), "bob");
    /// ```
    pub(super) fn normalize_identity(value: &str) -> String {
        value.trim().trim_start_matches('@').to_string()
    }

    /// 批量规范化允许用户列表
    ///
    /// 对用户列表进行规范化处理，包括：
    /// - 去除每个条目的 @ 前缀和空白
    /// - 过滤掉空字符串
    ///
    /// # 参数
    ///
    /// - `allowed_users`: 原始用户列表
    ///
    /// # 返回值
    ///
    /// 返回规范化后的用户列表
    pub(super) fn normalize_allowed_users(allowed_users: Vec<String>) -> Vec<String> {
        allowed_users
            .into_iter()
            .map(|entry| Self::normalize_identity(&entry))
            .filter(|entry| !entry.is_empty())
            .collect()
    }

    /// 规范化群聊回复允许发送者 ID 列表
    ///
    /// 对发送者 ID 列表进行规范化处理，包括：
    /// - 去除首尾空白
    /// - 过滤空字符串
    /// - 排序并去重
    ///
    /// # 参数
    ///
    /// - `sender_ids`: 原始发送者 ID 列表
    ///
    /// # 返回值
    ///
    /// 返回规范化、排序并去重后的发送者 ID 列表
    pub(super) fn normalize_group_reply_allowed_sender_ids(sender_ids: Vec<String>) -> Vec<String> {
        let mut normalized = sender_ids
            .into_iter()
            .map(|entry| entry.trim().to_string())
            .filter(|entry| !entry.is_empty())
            .collect::<Vec<_>>();
        // 排序以确保一致性和去重效果
        normalized.sort();
        normalized.dedup();
        normalized
    }

    /// 检查群聊发送者是否启用触发
    ///
    /// 判断指定的发送者是否在允许列表中，从而可以绕过 @提及 限制。
    ///
    /// # 参数
    ///
    /// - `sender_id`: 发送者 ID（可选）
    ///
    /// # 返回值
    ///
    /// - `true`: 发送者在允许列表中，或允许列表包含通配符 "*"
    /// - `false`: 发送者不在允许列表中，或 sender_id 为空
    pub(super) fn is_group_sender_trigger_enabled(&self, sender_id: Option<&str>) -> bool {
        // 检查 sender_id 是否有效（非空且非纯空白）
        let Some(sender_id) = sender_id.map(str::trim).filter(|id| !id.is_empty()) else {
            return false;
        };

        // 检查是否匹配列表中的任一条目（支持通配符 "*"）
        self.group_reply_allowed_sender_ids.iter().any(|entry| entry == "*" || entry == sender_id)
    }

    /// 从本地租户数据库加载配置（不读取环境变量）
    ///
    /// 此方法不处理环境变量覆盖，也不读取旧用户配置文件。
    ///
    /// # 返回值
    ///
    /// - `Ok(Config)`: 成功加载的配置对象
    /// - `Err`: 数据库读取、解密或解析失败
    ///
    /// # 错误
    ///
    /// - 无法找到用户主目录或打开共享数据库
    /// - 配置 payload 解密、反序列化或校验失败
    pub(super) async fn load_config_without_env() -> anyhow::Result<Config> {
        crate::app::agent::config::get_global_for_tenant_without_env(
            &vw_api_types::id::TenantScope::default(),
        )
        .await
        .map_err(anyhow::Error::from)
    }

    /// 持久化允许的用户身份到配置文件
    ///
    /// 将新绑定的用户身份添加到配置文件的 `allowed_users` 列表中。
    /// 如果用户已存在，则不会重复添加。
    ///
    /// # 参数
    ///
    /// - `identity`: 要持久化的用户身份（会自动规范化）
    ///
    /// # 返回值
    ///
    /// - `Ok(())`: 成功持久化或用户已存在
    /// - `Err`: 配置文件缺失必要节、无法写入或身份为空
    ///
    /// # 错误
    ///
    /// - 配置文件中缺少 `[channels.telegram]` 节
    /// - 身份字符串为空
    /// - 配置文件写入失败
    pub(super) async fn persist_allowed_identity(&self, identity: &str) -> anyhow::Result<()> {
        // 加载当前配置
        let mut config = Self::load_config_without_env().await?;

        // 检查 telegram 配置节是否存在
        let Some(telegram) = config.channels_config.telegram.as_mut() else {
            anyhow::bail!(
                "Missing channels.telegram in the system config. \
                Configure bot_token and allowed_users before binding"
            );
        };

        // 规范化身份并验证
        let normalized = Self::normalize_identity(identity);
        if normalized.is_empty() {
            anyhow::bail!("Cannot persist empty Telegram identity");
        }

        // 如果用户不在列表中，添加并保存
        if !telegram.allowed_users.iter().any(|u| u == &normalized) {
            telegram.allowed_users.push(normalized);
            crate::app::agent::config::schema::save_config(&config)
                .await
                .context("Failed to persist Telegram allowlist to the system config database")?;
        }

        Ok(())
    }

    /// 在运行时添加允许的用户身份
    ///
    /// 将用户身份添加到内存中的允许列表，不影响配置文件。
    /// 用于配对成功后的即时授权。
    ///
    /// # 参数
    ///
    /// - `identity`: 要添加的用户身份（会自动规范化）
    ///
    /// # 注意
    ///
    /// 此方法仅修改内存中的列表，需要配合 `persist_allowed_identity` 持久化
    pub(super) fn add_allowed_identity_runtime(&self, identity: &str) {
        let normalized = Self::normalize_identity(identity);
        // 忽略空身份
        if normalized.is_empty() {
            return;
        }
        // 获取写锁并添加用户（如果不存在）
        if let Ok(mut users) = self.allowed_users.write() {
            if !users.iter().any(|u| u == &normalized) {
                users.push(normalized);
            }
        }
    }

    /// 从消息文本中提取绑定码
    ///
    /// 解析 `/bind` 命令并提取其中的配对码。
    /// 支持带机器人用户名的命令格式（如 `/bind@mybot CODE`）。
    ///
    /// # 参数
    ///
    /// - `text`: 消息文本内容
    ///
    /// # 返回值
    ///
    /// - `Some(&str)`: 成功提取的配对码
    /// - `None`: 不是绑定命令或命令格式不正确
    ///
    /// # 示例
    ///
    /// ```ignore
    /// assert_eq!(TelegramChannel::extract_bind_code("/bind ABC123"), Some("ABC123"));
    /// assert_eq!(TelegramChannel::extract_bind_code("/bind@mybot XYZ789"), Some("XYZ789"));
    /// assert_eq!(TelegramChannel::extract_bind_code("/other command"), None);
    /// ```
    pub(super) fn extract_bind_code(text: &str) -> Option<&str> {
        let mut parts = text.split_whitespace();

        // 获取命令部分
        let command = parts.next()?;

        // 处理带机器人名的命令格式（如 /bind@mybot）
        let base_command = command.split('@').next().unwrap_or(command);

        // 验证是否为绑定命令
        if base_command != TELEGRAM_BIND_COMMAND {
            return None;
        }

        // 提取并验证配对码
        parts.next().map(str::trim).filter(|code| !code.is_empty())
    }

    /// 检查配对码是否处于活跃状态
    ///
    /// 判断当前是否存在有效的配对码可供绑定使用。
    ///
    /// # 返回值
    ///
    /// - `true`: 存在活跃的配对码
    /// - `false`: 配对码已过期、已使用或未启用配对模式
    pub(super) fn pairing_code_active(&self) -> bool {
        self.pairing.as_ref().and_then(PairingGuard::pairing_code).is_some()
            || self.allowed_users.read().is_ok_and(|users| users.is_empty())
    }

    /// 从 Telegram API 获取机器人用户名
    ///
    /// 调用 `getMe` API 端点获取机器人的用户名信息。
    ///
    /// # 返回值
    ///
    /// - `Ok(String)`: 机器人的用户名（不含 @ 符号）
    /// - `Err`: API 调用失败或响应解析失败
    pub(super) async fn fetch_bot_username(&self) -> anyhow::Result<String> {
        // 调用 getMe API
        let resp = self.http_client().get(self.api_url("getMe")).send().await?;

        // 检查 HTTP 状态
        if !resp.status().is_success() {
            anyhow::bail!("Failed to fetch bot info: {}", resp.status());
        }

        // 解析响应 JSON
        let data: serde_json::Value = resp.json().await?;

        // 提取用户名字段
        let username = data
            .get("result")
            .and_then(|r| r.get("username"))
            .and_then(|u| u.as_str())
            .context("Bot username not found in response")?;

        Ok(username.to_string())
    }

    /// 获取机器人用户名（带缓存）
    ///
    /// 获取机器人的用户名，优先使用缓存的值。
    /// 如果缓存为空，则从 API 获取并更新缓存。
    ///
    /// # 返回值
    ///
    /// - `Some(String)`: 成功获取的机器人用户名
    /// - `None`: 获取失败（会记录警告日志）
    ///
    /// # 线程安全
    ///
    /// 使用 Mutex 确保缓存访问的线程安全性
    pub(super) async fn get_bot_username(&self) -> Option<String> {
        // 首先检查缓存
        {
            let cache = self.bot_username.lock();
            if let Some(ref username) = *cache {
                return Some(username.clone());
            }
        }

        // 缓存未命中，从 API 获取
        match self.fetch_bot_username().await {
            Ok(username) => {
                // 更新缓存
                let mut cache = self.bot_username.lock();
                *cache = Some(username.clone());
                Some(username)
            }
            Err(e) => {
                // 记录警告但不中断执行
                tracing::warn!("Failed to fetch bot username: {e}");
                None
            }
        }
    }
}
