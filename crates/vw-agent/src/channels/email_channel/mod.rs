//! # 电子邮件通道模块
//!
//! 本模块实现了电子邮件通道（Email Channel），提供基于 IMAP IDLE 的实时邮件推送和 SMTP 发送功能。
//!
//! ## 核心功能
//!
//! - **IMAP IDLE 推送**：利用 IMAP IDLE 扩展实现即时新邮件通知，无需轮询
//! - **SMTP 发送**：支持通过 SMTP 协议发送邮件，支持 TLS 加密
//! - **发件人白名单**：支持基于邮箱地址和域名的发件人过滤
//! - **自动重连**：内置指数退避机制，网络中断后自动恢复连接
//!
//! ## 架构设计
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                     EmailChannel                             │
//! ├─────────────────────────────────────────────────────────────┤
//! │  IMAP IDLE 监听 ←→ 新邮件检测 ←→ 白名单过滤 → ChannelMessage │
//! │                          ↓                                    │
//! │                    ParsedEmail                                │
//! ├─────────────────────────────────────────────────────────────┤
//! │  SMTP 发送 ← SendMessage → 构建邮件 → 发送至收件人            │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## 使用示例
//!
//! ```rust,no_run
//! use vibe_agent::app::agent::channels::email_channel::EmailChannel;
//! use vibe_agent::app::agent::config::EmailConfig;
//!
//! // 创建邮件通道配置
//! let config = EmailConfig {
//!     imap_host: "imap.example.com".to_string(),
//!     imap_port: 993,
//!     smtp_host: "smtp.example.com".to_string(),
//!     smtp_port: 587,
//!     username: "user@example.com".to_string(),
//!     password: "password".to_string(),
//!     from_address: "noreply@example.com".to_string(),
//!     allowed_senders: vec!["@trusted.com".to_string()],
//!     idle_timeout_secs: 1740, // 29分钟，符合 RFC 2177 建议
//!     imap_folder: "INBOX".to_string(),
//!     smtp_tls: true,
//! };
//!
//! // 创建邮件通道实例
//! let channel = EmailChannel::new(config);
//! ```
//!
//! ## 注意事项
//!
//! - IMAP 服务器必须支持 IDLE 扩展
//! - 建议使用专用的应用专用密码而非账户主密码
//! - 长时间 IDLE 连接会被服务器断开，模块内置 29 分钟超时重连

#![allow(clippy::uninlined_format_args)]
#![allow(clippy::map_unwrap_or)]
#![allow(clippy::redundant_closure_for_method_calls)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::trim_split_whitespace)]
#![allow(clippy::doc_link_with_quotes)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::unnecessary_map_or)]

use anyhow::{Result, anyhow};
use async_imap::Session;
use async_imap::extensions::idle::IdleResponse;
use async_imap::types::Fetch;
use async_trait::async_trait;
use futures_util::TryStreamExt;
use lettre::message::SinglePart;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use mail_parser::{MessageParser, MimeHeaders};
use rustls::{ClientConfig, RootCertStore};
use rustls_pki_types::DnsName;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::net::TcpStream;
use tokio::sync::{Mutex, mpsc};
use tokio::time::{sleep, timeout};
use tokio_rustls::TlsConnector;
use tokio_rustls::client::TlsStream;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::traits::{Channel, ChannelMessage, SendMessage};
pub use crate::app::agent::config::EmailConfig;

/// IMAP 会话类型别名
///
/// 封装了 TLS 加密的 TCP 连接上的 IMAP 会话
type ImapSession = Session<TlsStream<TcpStream>>;

/// 电子邮件通道实现
///
/// 提供基于 IMAP IDLE 的实时邮件推送和 SMTP 发送功能。该结构体是 [`Channel`] trait 的具体实现，
/// 负责邮件的接收（通过 IMAP）和发送（通过 SMTP）。
///
/// # 字段说明
///
/// - `config`：电子邮件配置，包含 IMAP/SMTP 服务器信息、认证凭据和白名单等
/// - `seen_messages`：已处理邮件 ID 集合，使用 Arc<Mutex> 实现线程安全的去重机制
///
/// # 线程安全
///
/// `seen_messages` 字段使用 `Arc<Mutex<HashSet<String>>` 包装，支持多任务并发访问，
/// 确保同一封邮件不会被重复处理。
///
/// # 示例
///
/// ```rust,no_run
/// use vibe_agent::app::agent::channels::email_channel::EmailChannel;
/// use vibe_agent::app::agent::config::EmailConfig;
///
/// let config = EmailConfig {
///     // ... 配置参数
/// };
/// let channel = EmailChannel::new(config);
/// ```
pub struct EmailChannel {
    /// 电子邮件配置，包含服务器地址、认证信息和白名单
    pub config: EmailConfig,
    /// 已处理邮件 ID 集合，用于去重避免重复处理
    seen_messages: Arc<Mutex<HashSet<String>>>,
}

impl EmailChannel {
    /// 创建新的电子邮件通道实例
    ///
    /// # 参数
    ///
    /// - `config`：电子邮件配置，包含 IMAP/SMTP 服务器信息、认证凭据等
    ///
    /// # 返回值
    ///
    /// 返回初始化完成的 `EmailChannel` 实例，`seen_messages` 集合初始化为空
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use vibe_agent::app::agent::channels::email_channel::EmailChannel;
    /// use vibe_agent::app::agent::config::EmailConfig;
    ///
    /// let config = EmailConfig {
    ///     imap_host: "imap.gmail.com".to_string(),
    ///     // ... 其他配置
    /// };
    /// let channel = EmailChannel::new(config);
    /// ```
    pub fn new(config: EmailConfig) -> Self {
        Self { config, seen_messages: Arc::new(Mutex::new(HashSet::new())) }
    }

    /// 检查发件人邮箱是否在白名单中
    ///
    /// 支持三种白名单格式：
    /// - 通配符 `"*"`：允许所有发件人
    /// - 域名匹配 `"@example.com"` 或 `"example.com"`：允许该域名下所有邮箱
    /// - 完整邮箱地址 `"user@example.com"`：仅允许该特定邮箱
    ///
    /// # 参数
    ///
    /// - `email`：待检查的发件人邮箱地址
    ///
    /// # 返回值
    ///
    /// - `true`：发件人在白名单中，允许处理
    /// - `false`：发件人不在白名单中或白名单为空，拒绝处理
    ///
    /// # 安全策略
    ///
    /// - 白名单为空时默认拒绝所有发件人（安全优先原则）
    /// - 匹配时忽略大小写
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// // 假设 allowed_senders = vec!["@trusted.com", "admin@example.com"]
    /// assert!(channel.is_sender_allowed("user@trusted.com"));  // 域名匹配
    /// assert!(channel.is_sender_allowed("admin@example.com")); // 完整匹配
    /// assert!(!channel.is_sender_allowed("spam@evil.com"));    // 拒绝
    /// ```
    pub fn is_sender_allowed(&self, email: &str) -> bool {
        // 白名单为空时拒绝所有发件人（安全优先）
        if self.config.allowed_senders.is_empty() {
            return false;
        }
        // 通配符 "*" 表示允许所有发件人
        if self.config.allowed_senders.iter().any(|a| a == "*") {
            return true;
        }
        // 统一转换为小写进行匹配
        let email_lower = email.to_lowercase();
        self.config.allowed_senders.iter().any(|allowed| {
            if allowed.starts_with('@') {
                // 域名匹配（带 @ 前缀）："@example.com" 匹配 "user@example.com"
                email_lower.ends_with(&allowed.to_lowercase())
            } else if allowed.contains('@') {
                // 完整邮箱地址匹配（忽略大小写）
                allowed.eq_ignore_ascii_case(email)
            } else {
                // 域名匹配（不带 @ 前缀）："example.com" 匹配 "user@example.com"
                email_lower.ends_with(&format!("@{}", allowed.to_lowercase()))
            }
        })
    }

    /// 移除 HTML 标签并规范化空白字符
    ///
    /// 这是一个基础的 HTML 清理函数，用于从邮件正文中提取纯文本内容。
    /// 注意：此函数仅处理标签移除，不处理 HTML 实体编码转换。
    ///
    /// # 参数
    ///
    /// - `html`：包含 HTML 标签的原始字符串
    ///
    /// # 返回值
    ///
    /// 返回移除所有 HTML 标签后的纯文本字符串，连续空白字符合并为单个空格
    ///
    /// # 处理逻辑
    ///
    /// 1. 遍历字符，跳过 `<` 和 `>` 之间的所有内容（标签内容）
    /// 2. 保留标签外的所有文本内容
    /// 3. 规范化连续空白字符为单个空格
    ///
    /// # 示例
    ///
    /// ```
    /// let html = "<p>Hello <b>World</b>!</p>";
    /// let text = EmailChannel::strip_html(html);
    /// assert_eq!(text, "Hello World!");
    /// ```
    pub fn strip_html(html: &str) -> String {
        let mut result = String::new();
        let mut in_tag = false;
        // 第一遍：移除 HTML 标签
        for ch in html.chars() {
            match ch {
                '<' => in_tag = true,            // 进入标签
                '>' => in_tag = false,           // 退出标签
                _ if !in_tag => result.push(ch), // 标签外字符保留
                _ => {}                          // 标签内字符跳过
            }
        }
        // 第二遍：规范化连续空白字符
        let mut normalized = String::with_capacity(result.len());
        for word in result.split_whitespace() {
            if !normalized.is_empty() {
                normalized.push(' ');
            }
            normalized.push_str(word);
        }
        normalized
    }

    /// 从已解析的邮件中提取发件人地址
    ///
    /// # 参数
    ///
    /// - `parsed`：已解析的邮件对象（来自 mail_parser 库）
    ///
    /// # 返回值
    ///
    /// 返回发件人邮箱地址字符串。如果无法解析发件人信息，返回 `"unknown"`
    ///
    /// # 内部逻辑
    ///
    /// 依次尝试提取：From 字段 -> 第一个地址 -> 地址字符串
    fn extract_sender(parsed: &mail_parser::Message) -> String {
        parsed
            .from()
            .and_then(|addr| addr.first())
            .and_then(|a| a.address())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "unknown".into())
    }

    /// 从已解析的邮件中提取可读文本内容
    ///
    /// 按以下优先级尝试提取文本：
    /// 1. 纯文本正文（text/plain）
    /// 2. HTML 正文（text/html）- 移除标签后返回
    /// 3. 附件中的文本类型内容
    ///
    /// # 参数
    ///
    /// - `parsed`：已解析的邮件对象
    ///
    /// # 返回值
    ///
    /// 返回提取的文本内容字符串。如果无法提取任何可读内容，返回 `"(no readable content)"`
    fn extract_text(parsed: &mail_parser::Message) -> String {
        // 优先使用纯文本正文
        if let Some(text) = parsed.body_text(0) {
            if !text.contains('\u{fffd}') {
                return text.trim().to_string();
            }
        }
        // 其次使用 HTML 正文（移除标签）
        if let Some(html) = parsed.body_html(0) {
            return Self::strip_html(html.as_ref()).trim().to_string();
        }
        // 最后尝试从附件中提取文本
        for part in parsed.attachments() {
            let part: &mail_parser::MessagePart = part;
            if let Some(ct) = MimeHeaders::content_type(part) {
                if ct.ctype() == "text" {
                    if let Ok(text) = std::str::from_utf8(part.contents())
                        && !text.contains('\u{fffd}')
                    {
                        let name = MimeHeaders::attachment_name(part).unwrap_or("file");
                        return format!("[Attachment: {}]\n{}", name, text.trim());
                    }
                }
            }
        }
        "(no readable content)".to_string()
    }

    /// 连接到 IMAP 服务器并进行 TLS 认证
    ///
    /// 建立到 IMAP 服务器的安全连接，流程如下：
    /// 1. 建立 TCP 连接
    /// 2. 使用 rustls 进行 TLS 握手
    /// 3. 使用用户名和密码登录 IMAP 服务器
    ///
    /// # 返回值
    ///
    /// - `Ok(ImapSession)`：成功连接并认证后的 IMAP 会话
    /// - `Err`：连接或认证失败
    ///
    /// # 错误场景
    ///
    /// - 网络连接失败（DNS 解析、TCP 超时等）
    /// - TLS 握手失败（证书验证失败等）
    /// - IMAP 登录失败（认证凭据错误）
    ///
    /// # 安全说明
    ///
    /// - 使用系统信任的根证书进行 TLS 验证（webpki_roots）
    /// - 不使用客户端证书认证
    async fn connect_imap(&self) -> Result<ImapSession> {
        let addr = format!("{}:{}", self.config.imap_host, self.config.imap_port);
        debug!("Connecting to IMAP server at {}", addr);

        // 第一步：建立 TCP 连接
        let tcp = TcpStream::connect(&addr).await?;

        // 第二步：使用 rustls 建立 TLS 加密连接
        let certs = RootCertStore { roots: webpki_roots::TLS_SERVER_ROOTS.into() };
        let config = ClientConfig::builder().with_root_certificates(certs).with_no_client_auth();
        let tls_stream: TlsConnector = Arc::new(config).into();
        // SNI（Server Name Indication）用于 TLS 握手时传递主机名
        let sni: DnsName = self.config.imap_host.clone().try_into()?;
        let stream = tls_stream.connect(sni.into(), tcp).await?;

        // 第三步：创建 IMAP 客户端并登录
        let client = async_imap::Client::new(stream);

        // 使用配置的用户名和密码进行登录
        let session = client
            .login(&self.config.username, &self.config.password)
            .await
            .map_err(|(e, _)| anyhow!("IMAP login failed: {}", e))?;

        debug!("IMAP login successful");
        Ok(session)
    }

    /// 获取并处理邮箱中的未读邮件
    ///
    /// 从当前选中的邮箱中搜索所有未读（UNSEEN）邮件，解析其内容并返回结构化数据。
    ///
    /// # 参数
    ///
    /// - `session`：可变的 IMAP 会话引用，用于执行搜索和获取操作
    ///
    /// # 返回值
    ///
    /// 返回包含所有未读邮件解析结果的向量。如果没有未读邮件，返回空向量。
    ///
    /// # 处理流程
    ///
    /// 1. 使用 UID SEARCH UNSEEN 命令搜索未读邮件
    /// 2. 使用 UID FETCH RFC822 获取完整邮件内容
    /// 3. 解析邮件头和正文，提取发件人、主题、内容等
    /// 4. 将获取的邮件标记为已读（\Seen 标志）
    ///
    /// # 注意事项
    ///
    /// - 即使解析失败，邮件也会被标记为已读
    /// - 时间戳解析失败时使用当前系统时间
    async fn fetch_unseen(&self, session: &mut ImapSession) -> Result<Vec<ParsedEmail>> {
        // 搜索所有未读邮件的 UID
        let uids = session.uid_search("UNSEEN").await?;
        if uids.is_empty() {
            return Ok(Vec::new());
        }

        debug!("Found {} unseen messages", uids.len());

        let mut results = Vec::new();
        // 将 UID 列表转换为逗号分隔的字符串
        let uid_set: String = uids.iter().map(|u| u.to_string()).collect::<Vec<_>>().join(",");

        // 使用 UID FETCH 获取完整邮件内容（RFC822 格式）
        let messages = session.uid_fetch(&uid_set, "RFC822").await?;
        let messages: Vec<Fetch> = messages.try_collect().await?;

        for msg in messages {
            let uid = msg.uid.unwrap_or(0);
            if let Some(body) = msg.body() {
                let fallback_timestamp =
                    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
                if let Some(email) = Self::parsed_email_from_body(uid, body, fallback_timestamp) {
                    results.push(email);
                }
            }
        }

        // 将获取的邮件标记为已读（添加 \Seen 标志）
        if !results.is_empty() {
            let _ =
                session.uid_store(&uid_set, "+FLAGS (\\Seen)").await?.try_collect::<Vec<_>>().await;
        }

        Ok(results)
    }

    /// 等待 IMAP IDLE 响应（新邮件到达或超时）
    ///
    /// 进入 IMAP IDLE 模式，等待服务器推送新邮件通知或超时。
    /// 注意：IDLE 操作会消耗（consume）会话对象，在完成时通过 `done()` 返回。
    ///
    /// # 参数
    ///
    /// - `session`：IMAP 会话对象（所有权转移）
    ///
    /// # 返回值
    ///
    /// 返回元组 `(IdleWaitResult, ImapSession)`：
    /// - `IdleWaitResult`：等待结果类型（新邮件/超时/中断）
    /// - `ImapSession`：恢复正常模式的 IMAP 会话
    ///
    /// # IDLE 协议说明
    ///
    /// IMAP IDLE（RFC 2177）允许客户端保持连接并等待服务器主动推送新邮件通知，
    /// 而无需轮询。建议每 29 分钟重新建立 IDLE 连接以符合 RFC 规范。
    async fn wait_for_changes(
        &self,
        session: ImapSession,
    ) -> Result<(IdleWaitResult, ImapSession)> {
        let idle_timeout = Duration::from_secs(self.config.idle_timeout_secs);

        // 进入 IDLE 模式（消耗会话所有权）
        let mut idle = session.idle();
        idle.init().await?;

        debug!("Entering IMAP IDLE mode");

        // wait() 返回 (future, stop_source)，我们只需要 future
        let (wait_future, _stop_source) = idle.wait();

        // 等待服务器通知或超时
        let result = timeout(idle_timeout, wait_future).await;

        match result {
            Ok(Ok(response)) => {
                debug!("IDLE response: {:?}", response);
                // 完成 IDLE，恢复会话到正常模式
                let session = idle.done().await?;
                let wait_result = match response {
                    IdleResponse::NewData(_) => IdleWaitResult::NewMail,
                    IdleResponse::Timeout => IdleWaitResult::Timeout,
                    IdleResponse::ManualInterrupt => IdleWaitResult::Interrupted,
                };
                Ok((wait_result, session))
            }
            Ok(Err(e)) => {
                // 尝试清理 IDLE 状态
                let _ = idle.done().await;
                Err(anyhow!("IDLE error: {}", e))
            }
            Err(_) => {
                // 超时 - RFC 2177 建议每 29 分钟重启 IDLE
                debug!("IDLE timeout reached, will re-establish");
                let session = idle.done().await?;
                Ok((IdleWaitResult::Timeout, session))
            }
        }
    }

    /// 基于 IDLE 的主监听循环（带自动重连）
    ///
    /// 持续运行 IMAP IDLE 监听，当连接断开时使用指数退避策略自动重连。
    ///
    /// # 参数
    ///
    /// - `tx`：通道消息发送器，用于将接收到的邮件转发给上层处理
    ///
    /// # 返回值
    ///
    /// - `Ok(())`：通道关闭，正常退出
    /// - `Err`：无法恢复的错误
    ///
    /// # 重连策略
    ///
    /// - 初始退避时间：1 秒
    /// - 最大退避时间：60 秒
    /// - 指数增长：每次失败后翻倍，直到达到上限
    async fn listen_with_idle(&self, tx: mpsc::Sender<ChannelMessage>) -> Result<()> {
        let mut backoff = Duration::from_secs(1);
        let max_backoff = Duration::from_secs(60);

        loop {
            match self.run_idle_session(&tx).await {
                Ok(()) => {
                    // 通道关闭，正常退出
                    return Ok(());
                }
                Err(e) => {
                    error!("IMAP session error: {}. Reconnecting in {:?}...", e, backoff);
                    sleep(backoff).await;
                    // 指数退避，上限为 max_backoff
                    backoff = std::cmp::min(backoff * 2, max_backoff);
                }
            }
        }
    }

    /// 运行单次 IDLE 会话直到出错或干净退出
    ///
    /// 建立连接、选择邮箱、处理现有未读邮件，然后进入 IDLE 循环等待新邮件。
    ///
    /// # 参数
    ///
    /// - `tx`：通道消息发送器
    ///
    /// # 返回值
    ///
    /// - `Ok(())`：干净退出（如收到中断信号）
    /// - `Err`：连接错误，触发上层重连
    async fn run_idle_session(&self, tx: &mpsc::Sender<ChannelMessage>) -> Result<()> {
        // 连接并认证
        let mut session = self.connect_imap().await?;

        // 选择邮箱（如 INBOX）
        session.select(&self.config.imap_folder).await?;
        info!("Email IDLE listening on {} (instant push enabled)", self.config.imap_folder);

        // 首先处理现有的未读邮件
        self.process_unseen(&mut session, tx).await?;

        loop {
            // 进入 IDLE 等待变化（消耗会话，通过结果返回）
            match self.wait_for_changes(session).await {
                Ok((IdleWaitResult::NewMail, returned_session)) => {
                    debug!("New mail notification received");
                    session = returned_session;
                    // 处理新到达的邮件
                    self.process_unseen(&mut session, tx).await?;
                }
                Ok((IdleWaitResult::Timeout, returned_session)) => {
                    // IDLE 超时后重新检查（防御性措施）
                    session = returned_session;
                    self.process_unseen(&mut session, tx).await?;
                }
                Ok((IdleWaitResult::Interrupted, _)) => {
                    info!("IDLE interrupted, exiting");
                    return Ok(());
                }
                Err(e) => {
                    // 连接可能已断开，需要重连
                    return Err(e);
                }
            }
        }
    }

    /// 获取未读邮件并发送到通道
    ///
    /// 从 IMAP 服务器获取未读邮件，经过白名单过滤和去重后，将有效邮件转换为
    /// `ChannelMessage` 并发送到消息通道。
    ///
    /// # 参数
    ///
    /// - `session`：可变的 IMAP 会话引用
    /// - `tx`：通道消息发送器
    ///
    /// # 返回值
    ///
    /// - `Ok(())`：处理完成
    /// - `Err`：IMAP 操作错误
    ///
    /// # 处理流程
    ///
    /// 1. 获取未读邮件列表
    /// 2. 检查发件人是否在白名单中
    /// 3. 检查邮件是否已处理过（基于 msg_id 去重）
    /// 4. 构造 ChannelMessage 并发送
    async fn process_unseen(
        &self,
        session: &mut ImapSession,
        tx: &mpsc::Sender<ChannelMessage>,
    ) -> Result<()> {
        let messages = self.fetch_unseen(session).await?;

        for email in messages {
            // 白名单检查：拒绝不在白名单中的发件人
            if !self.is_sender_allowed(&email.sender) {
                warn!("Blocked email from {}", email.sender);
                continue;
            }

            // 去重检查：使用 msg_id 确保每封邮件只处理一次
            let is_new = {
                let mut seen = self.seen_messages.lock().await;
                seen.insert(email.msg_id.clone())
            };
            if !is_new {
                continue;
            }

            // 构造通道消息
            let msg = ChannelMessage {
                id: email.msg_id,
                reply_target: email.sender.clone(),
                sender: email.sender,
                content: email.content,
                channel: "email".to_string(),
                timestamp: email.timestamp,
                thread_ts: None,
            };

            // 发送消息，如果通道已关闭则退出
            if tx.send(msg).await.is_err() {
                return Ok(());
            }
        }

        Ok(())
    }

    /// 创建 SMTP 传输实例
    ///
    /// 根据配置创建 SMTP 客户端传输，支持 TLS 加密和明文两种模式。
    ///
    /// # 返回值
    ///
    /// - `Ok(SmtpTransport)`：可用的 SMTP 传输实例
    /// - `Err`：配置错误或连接失败
    ///
    /// # 安全说明
    ///
    /// - `smtp_tls = true`：使用 STARTTLS 加密（推荐）
    /// - `smtp_tls = false`：明文传输（仅用于测试或不安全的内网环境）
    fn create_smtp_transport(&self) -> Result<SmtpTransport> {
        anyhow::ensure!(
            !self.config.smtp_host.is_empty()
                && !self.config.smtp_host.chars().any(char::is_whitespace),
            "SMTP host must be a non-empty hostname without whitespace"
        );
        let creds = Credentials::new(self.config.username.clone(), self.config.password.clone());
        let transport = if self.config.smtp_tls {
            // TLS 模式：使用安全的 SMTP 中继
            SmtpTransport::relay(&self.config.smtp_host)?
                .port(self.config.smtp_port)
                .credentials(creds)
                .build()
        } else {
            // 明文模式：不安全的连接（仅用于测试）
            SmtpTransport::builder_dangerous(&self.config.smtp_host)
                .port(self.config.smtp_port)
                .credentials(creds)
                .build()
        };
        Ok(transport)
    }

    fn parsed_email_from_body(
        uid: u32,
        body: &[u8],
        fallback_timestamp: u64,
    ) -> Option<ParsedEmail> {
        let parsed = MessageParser::default().parse(body)?;
        let sender = Self::extract_sender(&parsed);
        let subject = parsed.subject().unwrap_or("(no subject)").to_string();
        let body_text = Self::extract_text(&parsed);
        let content = format!("Subject: {}\n\n{}", subject, body_text);
        let msg_id = parsed
            .message_id()
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("gen-{}", Uuid::new_v4()));

        #[allow(clippy::cast_sign_loss)]
        let timestamp = parsed
            .date()
            .map(|d| {
                let naive = chrono::NaiveDate::from_ymd_opt(
                    d.year as i32,
                    u32::from(d.month),
                    u32::from(d.day),
                )
                .and_then(|date| {
                    date.and_hms_opt(u32::from(d.hour), u32::from(d.minute), u32::from(d.second))
                });
                naive.map_or(0, |n| n.and_utc().timestamp() as u64)
            })
            .unwrap_or(fallback_timestamp);

        Some(ParsedEmail { _uid: uid, msg_id, sender, content, timestamp })
    }

    fn outgoing_subject_and_body(message: &SendMessage) -> (&str, &str) {
        if let Some(ref subj) = message.subject {
            (subj.as_str(), message.content.as_str())
        } else if message.content.starts_with("Subject: ") {
            if let Some(pos) = message.content.find('\n') {
                (&message.content[9..pos], message.content[pos + 1..].trim())
            } else {
                ("VibeWindow Message", message.content.as_str())
            }
        } else {
            ("VibeWindow Message", message.content.as_str())
        }
    }

    fn build_outgoing_email(&self, message: &SendMessage) -> Result<Message> {
        let (subject, body) = Self::outgoing_subject_and_body(message);
        Ok(Message::builder()
            .from(self.config.from_address.parse()?)
            .to(message.recipient.parse()?)
            .subject(subject)
            .singlepart(SinglePart::plain(body.to_string()))?)
    }
}

/// 已解析的邮件数据结构
///
/// 存储从 IMAP 服务器获取并解析后的邮件信息，用于内部处理和转发。
///
/// # 字段说明
///
/// - `_uid`：IMAP 邮件的唯一标识符（目前未使用，保留用于未来扩展）
/// - `msg_id`：邮件的 Message-ID，用于去重
/// - `sender`：发件人邮箱地址
/// - `content`：邮件内容（包含主题和正文）
/// - `timestamp`：邮件时间戳（Unix 时间戳，秒）
struct ParsedEmail {
    /// IMAP 邮件 UID（未使用，保留用于未来扩展）
    _uid: u32,
    /// 邮件 Message-ID，用于去重标识
    msg_id: String,
    /// 发件人邮箱地址
    sender: String,
    /// 邮件内容（格式："Subject: xxx\n\nbody"）
    content: String,
    /// 邮件时间戳（Unix 时间戳，秒）
    timestamp: u64,
}

/// IDLE 等待结果枚举
///
/// 表示 IMAP IDLE 模式下的等待结果类型。
///
/// # 变体说明
///
/// - `NewMail`：服务器通知有新邮件到达
/// - `Timeout`：IDLE 超时（RFC 2177 建议定期重启 IDLE）
/// - `Interrupted`：IDLE 被手动中断（如收到关闭信号）
enum IdleWaitResult {
    /// 新邮件到达通知
    NewMail,
    /// IDLE 超时，需要重新建立
    Timeout,
    /// 被手动中断
    Interrupted,
}

/// 为 EmailChannel 实现 Channel trait
///
/// 实现 [`Channel`] trait 以集成到代理通道系统中，提供：
/// - `name`：通道标识符
/// - `send`：通过 SMTP 发送邮件
/// - `listen`：通过 IMAP IDLE 监听新邮件
/// - `health_check`：检查 IMAP 连接健康状态
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Channel for EmailChannel {
    /// 返回通道名称标识符
    ///
    /// # 返回值
    ///
    /// 固定返回 `"email"`，用于通道类型识别和路由
    fn name(&self) -> &str {
        "email"
    }

    /// 通过 SMTP 发送邮件
    ///
    /// 将消息发送给指定的收件人。支持显式指定主题或从内容中解析主题。
    ///
    /// # 参数
    ///
    /// - `message`：发送消息结构，包含收件人、内容和可选主题
    ///
    /// # 返回值
    ///
    /// - `Ok(())`：邮件发送成功
    /// - `Err`：发送失败（地址解析错误、SMTP 连接错误等）
    ///
    /// # 主题解析逻辑
    ///
    /// 1. 如果 `message.subject` 存在，使用该值作为主题
    /// 2. 如果内容以 `"Subject: "` 开头，尝试解析第一行为主题
    /// 3. 否则使用默认主题 `"VibeWindow Message"`
    async fn send(&self, message: &SendMessage) -> Result<()> {
        let email = self.build_outgoing_email(message)?;

        // 创建 SMTP 传输并发送
        let transport = self.create_smtp_transport()?;
        transport.send(&email)?;
        info!("Email sent to {}", message.recipient);
        Ok(())
    }

    /// 启动 IMAP IDLE 监听
    ///
    /// 开始监听新邮件，通过 IMAP IDLE 实现即时推送。此方法会阻塞运行，
    /// 直到通道关闭或发生无法恢复的错误。
    ///
    /// # 参数
    ///
    /// - `tx`：通道消息发送器，用于将接收到的邮件转发给代理处理
    ///
    /// # 返回值
    ///
    /// - `Ok(())`：通道关闭，正常退出
    /// - `Err`：IMAP 连接错误
    async fn listen(&self, tx: mpsc::Sender<ChannelMessage>) -> Result<()> {
        info!("Starting email channel with IDLE support on {}", self.config.imap_folder);
        self.listen_with_idle(tx).await
    }

    /// 执行健康检查
    ///
    /// 尝试连接到 IMAP 服务器并验证认证是否成功。
    ///
    /// # 返回值
    ///
    /// - `true`：IMAP 连接和认证成功
    /// - `false`：连接失败、认证失败或超时
    ///
    /// # 超时设置
    ///
    /// 健康检查超时时间为 10 秒，避免长时间阻塞。
    async fn health_check(&self) -> bool {
        // 全异步健康检查：尝试建立 IMAP 连接
        match timeout(Duration::from_secs(10), self.connect_imap()).await {
            Ok(Ok(mut session)) => {
                // 尝试正常登出
                let _ = session.logout().await;
                true
            }
            Ok(Err(e)) => {
                debug!("Health check failed: {}", e);
                false
            }
            Err(_) => {
                debug!("Health check timed out");
                false
            }
        }
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
