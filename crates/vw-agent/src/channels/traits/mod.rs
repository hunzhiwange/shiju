//! 通道 traits 模块
//!
//! 本模块定义了 VibeWindow 代理系统中通道集成的核心 trait 和数据结构。
//! 通道是代理与外部消息平台（如 Slack、Discord、Telegram 等）进行通信的抽象层。
//!
//! # 核心组件
//!
//! - [`Channel`] - 所有通道实现必须遵循的核心 trait，定义了发送、接收、健康检查等能力
//! - [`ChannelMessage`] - 从通道接收或发送到通道的消息表示
//! - [`SendMessage`] - 用于构造发送消息的结构体
//! - [`ChannelBounds`] - 跨平台（包括 WASM）的 trait 边界辅助
//!
//! # 架构设计
//!
//! 该模块采用 trait 驱动设计，允许通过实现 `Channel` trait 来添加新的消息平台支持。
//! 新的实现只需在工厂模块中注册即可被系统自动发现和使用。
//!
//! # 示例
//!
//! ```ignore
//! use crate::app::agent::channels::traits::{Channel, SendMessage, ChannelMessage};
//!
//! struct MyChannel {
//!     name: String,
//! }
//!
//! #[async_trait]
//! impl Channel for MyChannel {
//!     fn name(&self) -> &str {
//!         &self.name
//!     }
//!
//!     async fn send(&self, message: &SendMessage) -> anyhow::Result<()> {
//!         // 实现发送逻辑
//!         Ok(())
//!     }
//!
//!     async fn listen(&self, tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> anyhow::Result<()> {
//!         // 实现监听逻辑
//!         Ok(())
//!     }
//! }
//! ```

use async_trait::async_trait;

/// 通道消息 - 从通道接收或发送到通道的消息表示
///
/// 该结构体封装了消息的所有关键信息，包括发送者、内容、时间戳以及
/// 平台特定的线程标识符。它是代理系统处理消息的基本数据单元。
///
/// # 字段说明
///
/// - `id`: 消息的唯一标识符
/// - `sender`: 消息发送者的标识符
/// - `reply_target`: 回复目标标识符，用于确定消息应回复到哪里
/// - `content`: 消息的文本内容
/// - `channel`: 消息所属的通道标识符
/// - `timestamp`: 消息的时间戳（Unix 时间戳，毫秒）
/// - `thread_ts`: 可选的平台线程标识符，用于支持线程回复
///
/// # 示例
///
/// ```ignore
/// let msg = ChannelMessage {
///     id: "msg_123".to_string(),
///     sender: "user_456".to_string(),
///     reply_target: "channel_789".to_string(),
///     content: "你好，代理！".to_string(),
///     channel: "general".to_string(),
///     timestamp: 1700000000000,
///     thread_ts: Some("1234567890.123456".to_string()),
/// };
/// ```
#[derive(Debug, Clone)]
pub struct ChannelMessage {
    /// 消息的唯一标识符
    pub id: String,
    /// 消息发送者的标识符
    pub sender: String,
    /// 回复目标标识符
    pub reply_target: String,
    /// 消息的文本内容
    pub content: String,
    /// 消息所属的通道标识符
    pub channel: String,
    /// 消息时间戳（Unix 时间戳，毫秒）
    pub timestamp: u64,
    /// 平台线程标识符（如 Slack 的 `ts`、Discord 的线程 ID）
    ///
    /// 当设置了此字段时，回复应作为线程响应发布，而不是作为新消息。
    /// 这允许在支持线程的平台（如 Slack、Discord）中保持对话上下文。
    pub thread_ts: Option<String>,
}

/// 发送消息 - 用于构造要通过通道发送的消息
///
/// 该结构体提供了构造发送消息的便捷方式，支持设置内容、收件人、
/// 主题以及线程标识符。使用 builder 模式可以灵活地构建消息。
///
/// # 字段说明
///
/// - `content`: 消息的文本内容
/// - `recipient`: 消息接收者的标识符
/// - `subject`: 可选的消息主题（用于支持主题的平台）
/// - `thread_ts`: 可选的线程标识符，用于在线程中回复
///
/// # 示例
///
/// ```ignore
/// // 创建简单消息
/// let msg = SendMessage::new("你好！", "user_123");
///
/// // 创建带主题的消息
/// let msg = SendMessage::with_subject("内容", "user_123", "重要通知");
///
/// // 创建线程回复
/// let msg = SendMessage::new("回复内容", "user_123")
///     .in_thread(Some("1234567890.123456".to_string()));
/// ```
#[derive(Debug, Clone)]
pub struct SendMessage {
    /// 消息的文本内容
    pub content: String,
    /// 消息接收者的标识符
    pub recipient: String,
    /// 可选的消息主题
    pub subject: Option<String>,
    /// 平台线程标识符，用于线程回复（如 Slack 的 `thread_ts`）
    pub thread_ts: Option<String>,
}

impl SendMessage {
    /// 创建包含内容和收件人的新消息
    ///
    /// 这是最基本的消息构造方法，仅设置必需的内容和收件人字段。
    /// 其他可选字段（如主题、线程标识符）将设置为 `None`。
    ///
    /// # 参数
    ///
    /// - `content`: 消息内容，可以是任何实现了 `Into<String>` 的类型
    /// - `recipient`: 收件人标识符，可以是任何实现了 `Into<String>` 的类型
    ///
    /// # 返回值
    ///
    /// 返回一个新的 `SendMessage` 实例，其中 `subject` 和 `thread_ts` 为 `None`
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let msg = SendMessage::new("你好，世界！", "channel_general");
    /// ```
    pub fn new(content: impl Into<String>, recipient: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            recipient: recipient.into(),
            subject: None,
            thread_ts: None,
        }
    }

    /// 创建包含内容、收件人和主题的新消息
    ///
    /// 该方法在基本消息的基础上增加了主题字段，适用于支持
    /// 主题的消息平台（如邮件、某些论坛系统）。
    ///
    /// # 参数
    ///
    /// - `content`: 消息内容，可以是任何实现了 `Into<String>` 的类型
    /// - `recipient`: 收件人标识符，可以是任何实现了 `Into<String>` 的类型
    /// - `subject`: 消息主题，可以是任何实现了 `Into<String>` 的类型
    ///
    /// # 返回值
    ///
    /// 返回一个新的 `SendMessage` 实例，其中 `thread_ts` 为 `None`
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let msg = SendMessage::with_subject(
    ///     "这是消息内容",
    ///     "user_456",
    ///     "关于项目更新"
    /// );
    /// ```
    pub fn with_subject(
        content: impl Into<String>,
        recipient: impl Into<String>,
        subject: impl Into<String>,
    ) -> Self {
        Self {
            content: content.into(),
            recipient: recipient.into(),
            subject: Some(subject.into()),
            thread_ts: None,
        }
    }

    /// 设置线程标识符，用于在线程中回复
    ///
    /// 该方法使用 builder 模式，允许在现有消息上设置线程标识符。
    /// 当设置了 `thread_ts` 时，消息将作为对指定线程的回复发送。
    ///
    /// # 参数
    ///
    /// - `thread_ts`: 可选的线程标识符。`Some(...)` 表示在指定线程中回复，
    ///   `None` 表示不使用线程
    ///
    /// # 返回值
    ///
    /// 返回修改后的 `SendMessage` 实例（移动语义）
    ///
    /// # 示例
    ///
    /// ```ignore
    /// // 在指定线程中回复
    /// let msg = SendMessage::new("回复", "user_123")
    ///     .in_thread(Some("1234567890.123456".to_string()));
    ///
    /// // 不使用线程
    /// let msg = SendMessage::new("新消息", "user_123")
    ///     .in_thread(None);
    /// ```
    pub fn in_thread(mut self, thread_ts: Option<String>) -> Self {
        self.thread_ts = thread_ts;
        self
    }
}

/// 通道边界 trait - 用于定义跨平台的 trait 边界
///
/// 该 trait 是一个辅助 trait，用于在不同平台上统一 `Channel` trait 的边界要求。
/// 在非 WASM 平台上，要求实现 `Send + Sync`；在 WASM 平台上，不要求这些边界。
///
/// # 平台差异
///
/// - **非 WASM 平台**: 要求 `Send + Sync`，支持多线程环境
/// - **WASM 平台**: 不要求 `Send + Sync`，适应 WASM 的单线程模型
///
/// # 实现说明
///
/// 该 trait 使用条件编译自动为所有满足条件的类型提供实现，
/// 用户无需手动实现此 trait。
#[cfg(not(target_arch = "wasm32"))]
pub trait ChannelBounds: Send + Sync {}
#[cfg(not(target_arch = "wasm32"))]
impl<T: Send + Sync> ChannelBounds for T {}

/// 通道边界 trait - 用于定义跨平台的 trait 边界
///
/// 该 trait 是一个辅助 trait，用于在不同平台上统一 `Channel` trait 的边界要求。
/// 在非 WASM 平台上，要求实现 `Send + Sync`；在 WASM 平台上，不要求这些边界。
///
/// # 平台差异
///
/// - **非 WASM 平台**: 要求 `Send + Sync`，支持多线程环境
/// - **WASM 平台**: 不要求 `Send + Sync`，适应 WASM 的单线程模型
///
/// # 实现说明
///
/// 该 trait 使用条件编译自动为所有满足条件的类型提供实现，
/// 用户无需手动实现此 trait。
#[cfg(target_arch = "wasm32")]
pub trait ChannelBounds {}
#[cfg(target_arch = "wasm32")]
impl<T> ChannelBounds for T {}

/// 通道核心 trait - 所有消息平台的统一接口
///
/// 该 trait 定义了代理系统与外部消息平台交互的核心契约。
/// 任何希望集成到 VibeWindow 系统的消息平台都必须实现此 trait。
///
/// # 核心能力
///
/// - **消息发送**: 通过 `send` 方法发送消息
/// - **消息监听**: 通过 `listen` 方法持续监听入站消息
/// - **健康检查**: 通过 `health_check` 方法检查通道状态
/// - **状态指示**: 通过 `start_typing`/`stop_typing` 显示处理状态
/// - **草稿消息**: 支持渐进式消息更新（如流式输出）
/// - **审批提示**: 发送需要用户批准的交互式提示
/// - **消息反应**: 添加/移除消息的 emoji 反应
///
/// # 实现要求
///
/// - 必须实现 `name`、`send` 和 `listen` 方法
/// - 其他方法有默认实现，可根据平台能力选择性覆盖
/// - 实现应考虑平台的特性和限制（如消息长度限制、格式支持等）
///
/// # 平台适配示例
///
/// ```ignore
/// struct SlackChannel {
///     client: SlackClient,
///     token: String,
/// }
///
/// #[async_trait]
/// impl Channel for SlackChannel {
///     fn name(&self) -> &str {
///         "slack"
///     }
///
///     async fn send(&self, message: &SendMessage) -> anyhow::Result<()> {
///         self.client.post_message(&message.recipient, &message.content).await
///     }
///
///     async fn listen(&self, tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> anyhow::Result<()> {
///         self.client.start_rtm(&tx).await
///     }
///
///     fn supports_draft_updates(&self) -> bool {
///         true  // Slack 支持消息编辑
///     }
/// }
/// ```
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait Channel: ChannelBounds {
    /// 获取通道的可读名称
    ///
    /// 该方法返回用于日志、调试和用户界面显示的通道名称。
    ///
    /// # 返回值
    ///
    /// 返回通道的名称字符串切片，如 "slack"、"discord"、"telegram" 等
    fn name(&self) -> &str;

    /// 通过此通道发送消息
    ///
    /// 该方法是通道的核心功能，负责将消息投递到目标平台。
    /// 实现应处理平台特定的错误并转换为 `anyhow::Result`。
    ///
    /// # 参数
    ///
    /// - `message`: 要发送的消息，包含内容、收件人、主题等信息
    ///
    /// # 返回值
    ///
    /// - `Ok(())`: 消息发送成功
    /// - `Err(...)`: 发送失败，包含错误信息
    ///
    /// # 错误处理
    ///
    /// 实现应捕获平台特定错误并转换为 `anyhow::Error`，
    /// 提供有意义的错误信息以帮助调试。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let msg = SendMessage::new("测试消息", "channel_id");
    /// channel.send(&msg).await?;
    /// ```
    async fn send(&self, message: &SendMessage) -> anyhow::Result<()>;

    /// 开始监听入站消息（长时间运行）
    ///
    /// 该方法启动一个持续运行的任务，监听来自通道的入站消息，
    /// 并通过提供的通道发送器将消息传递给代理系统。
    ///
    /// # 参数
    ///
    /// - `tx`: 异步消息发送器，用于将接收到的消息发送到代理系统
    ///
    /// # 返回值
    ///
    /// - `Ok(())`: 监听器正常启动
    /// - `Err(...)`: 启动失败
    ///
    /// # 实现说明
    ///
    /// - 该方法通常会阻塞当前任务，应在独立的 tokio 任务中运行
    /// - 应处理连接断开并尝试自动重连
    /// - 应正确处理背压，避免消息积压
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let (tx, rx) = tokio::sync::mpsc::channel(100);
    /// tokio::spawn(async move {
    ///     channel.listen(tx).await
    /// });
    /// ```
    async fn listen(&self, tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> anyhow::Result<()>;

    /// 检查通道是否健康
    ///
    /// 该方法用于健康检查，判断通道是否可以正常工作。
    /// 默认实现返回 `true`，实现者可根据需要覆盖。
    ///
    /// # 返回值
    ///
    /// - `true`: 通道健康，可以正常工作
    /// - `false`: 通道不健康，可能需要修复或重启
    ///
    /// # 实现建议
    ///
    /// - 检查网络连接状态
    /// - 验证认证令牌是否有效
    /// - 可执行轻量级的 ping 操作
    ///
    /// # 示例
    ///
    /// ```ignore
    /// if !channel.health_check().await {
    ///     log::warn!("通道 {} 不健康", channel.name());
    /// }
    /// ```
    async fn health_check(&self) -> bool {
        true
    }

    /// 发出正在处理响应的信号（如"正在输入"指示器）
    ///
    /// 该方法用于在代理处理请求时向用户显示状态指示器。
    /// 实现应根据平台特性重复发送指示器（如每几秒发送一次）。
    ///
    /// # 参数
    ///
    /// - `_recipient`: 接收者的标识符
    ///
    /// # 返回值
    ///
    /// - `Ok(())`: 指示器启动成功
    /// - `Err(...)`: 启动失败（如果平台不支持，可忽略错误）
    ///
    /// # 平台支持
    ///
    /// - **Slack**: 显示"正在输入..."
    /// - **Discord**: 显示"正在输入"状态
    /// - **Telegram**: 显示"正在输入"状态
    /// - 其他不支持的平台应返回 `Ok(())`
    ///
    /// # 示例
    ///
    /// ```ignore
    /// channel.start_typing("user_123").await?;
    /// // 执行处理...
    /// channel.stop_typing("user_123").await?;
    /// ```
    async fn start_typing(&self, _recipient: &str) -> anyhow::Result<()> {
        Ok(())
    }

    /// 停止活动的"正在输入"指示器
    ///
    /// 该方法用于停止由 `start_typing` 启动的状态指示器。
    /// 应在消息发送完成或处理失败时调用。
    ///
    /// # 参数
    ///
    /// - `_recipient`: 接收者的标识符
    ///
    /// # 返回值
    ///
    /// - `Ok(())`: 指示器停止成功
    /// - `Err(...)`: 停止失败
    async fn stop_typing(&self, _recipient: &str) -> anyhow::Result<()> {
        Ok(())
    }

    /// 判断此通道是否支持通过草稿编辑进行渐进式消息更新
    ///
    /// 某些平台（如 Slack）支持编辑已发送的消息，这允许代理
    /// 在生成响应时逐步更新消息内容，而不是等待完整响应。
    ///
    /// # 返回值
    ///
    /// - `true`: 支持草稿更新（应实现 `send_draft`、`update_draft` 等方法）
    /// - `false`: 不支持（默认值）
    ///
    /// # 使用场景
    ///
    /// - 流式文本生成时逐步显示内容
    /// - 减少用户等待感知
    /// - 提供实时反馈
    fn supports_draft_updates(&self) -> bool {
        false
    }

    /// 发送初始草稿消息
    ///
    /// 该方法发送一个初始草稿消息，并返回平台特定的消息 ID，
    /// 用于后续的编辑操作。仅在 `supports_draft_updates` 返回 `true` 时使用。
    ///
    /// # 参数
    ///
    /// - `_message`: 要发送的草稿消息
    ///
    /// # 返回值
    ///
    /// - `Ok(Some(message_id))`: 成功发送草稿，返回消息 ID
    /// - `Ok(None)`: 发送失败或不支持（默认实现）
    /// - `Err(...)`: 发送过程中出错
    ///
    /// # 实现说明
    ///
    /// - 应返回可用于后续 `update_draft` 调用的消息 ID
    /// - 消息 ID 应该是平台原生的标识符
    async fn send_draft(&self, _message: &SendMessage) -> anyhow::Result<Option<String>> {
        Ok(None)
    }

    /// 使用累积的新内容更新之前发送的草稿消息
    ///
    /// 该方法用于更新草稿消息的内容，适用于流式生成场景。
    /// 某些平台有编辑次数限制，超出后可能需要创建续接消息。
    ///
    /// # 参数
    ///
    /// - `_recipient`: 收件人标识符
    /// - `_message_id`: 要更新的消息 ID（由 `send_draft` 返回）
    /// - `_text`: 新的消息文本内容
    ///
    /// # 返回值
    ///
    /// - `Ok(None)`: 更新成功，保持当前消息 ID
    /// - `Ok(Some(new_id))`: 创建了续接消息，返回新消息 ID
    ///   （例如：达到平台编辑次数限制后）
    /// - `Err(...)`: 更新失败
    ///
    /// # 平台限制
    ///
    /// - **Slack**: 对同一消息的编辑次数有限制
    /// - **Discord**: 消息内容超过限制时需要分割
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let mut current_id = initial_id;
    /// for chunk in stream {
    ///     current_id = channel.update_draft(recipient, &current_id, &chunk).await?;
    /// }
    /// ```
    async fn update_draft(
        &self,
        _recipient: &str,
        _message_id: &str,
        _text: &str,
    ) -> anyhow::Result<Option<String>> {
        Ok(None)
    }

    /// 使用完整的响应内容定稿草稿消息（如应用 Markdown 格式）
    ///
    /// 该方法在代理完成响应生成后调用，用于最终格式化消息。
    /// 可用于应用最终格式、添加格式化内容等。
    ///
    /// # 参数
    ///
    /// - `_recipient`: 收件人标识符
    /// - `_message_id`: 消息 ID
    /// - `_text`: 完整的消息文本内容
    ///
    /// # 返回值
    ///
    /// - `Ok(())`: 定稿成功
    /// - `Err(...)`: 定稿失败
    ///
    /// # 实现建议
    ///
    /// - 应用 Markdown 或平台特定的格式化
    /// - 添加最终装饰（如分隔线、签名等）
    async fn finalize_draft(
        &self,
        _recipient: &str,
        _message_id: &str,
        _text: &str,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    /// 取消并删除之前发送的草稿消息（如果通道支持）
    ///
    /// 该方法用于取消正在进行的草稿消息，例如在生成过程中
    /// 发生错误或用户取消操作时。
    ///
    /// # 参数
    ///
    /// - `_recipient`: 收件人标识符
    /// - `_message_id`: 要取消的消息 ID
    ///
    /// # 返回值
    ///
    /// - `Ok(())`: 取消成功或不支持（默认实现）
    /// - `Err(...)`: 取消失败
    async fn cancel_draft(&self, _recipient: &str, _message_id: &str) -> anyhow::Result<()> {
        Ok(())
    }

    /// 发送交互式审批提示（如果通道支持）
    ///
    /// 该方法发送一个需要用户批准的交互式提示，用于工具执行前的审批流程。
    /// 默认实现发送纯文本回退消息，包含斜杠命令操作指引。
    ///
    /// # 参数
    ///
    /// - `recipient`: 收件人标识符
    /// - `request_id`: 审批请求的唯一标识符
    /// - `tool_name`: 需要审批的工具名称
    /// - `arguments`: 工具的参数（JSON 格式）
    /// - `thread_ts`: 可选的线程标识符，用于在线程中发送提示
    ///
    /// # 返回值
    ///
    /// - `Ok(())`: 提示发送成功
    /// - `Err(...)`: 发送失败
    ///
    /// # 默认行为
    ///
    /// 默认实现构造一个包含审批和拒绝命令的纯文本消息，
    /// 适用于不支持交互式按钮的平台。
    ///
    /// # 平台支持
    ///
    /// - **Slack**: 可覆盖此方法使用 Block Kit 按钮实现交互式审批
    /// - **Discord**: 可使用消息组件按钮
    /// - 其他平台: 使用默认的文本回退
    ///
    /// # 示例
    ///
    /// ```ignore
    /// channel.send_approval_prompt(
    ///     "user_123",
    ///     "req_789",
    ///     "file_write",
    ///     &json!({"path": "/tmp/test.txt", "content": "hello"}),
    ///     Some("thread_456".to_string()),
    /// ).await?;
    /// ```
    async fn send_approval_prompt(
        &self,
        recipient: &str,
        request_id: &str,
        tool_name: &str,
        arguments: &serde_json::Value,
        thread_ts: Option<String>,
    ) -> anyhow::Result<()> {
        let raw_args = arguments.to_string();
        // 限制参数预览长度，避免消息过长
        let args_preview =
            if raw_args.len() > 220 { format!("{}...", &raw_args[..220]) } else { raw_args };
        let message = format!(
            "Approval required for tool `{tool_name}`.\nRequest ID: `{request_id}`\nArgs: `{args_preview}`\nApprove: `/approve-allow {request_id}`\nDeny: `/approve-deny {request_id}`"
        );
        self.send(&SendMessage::new(message, recipient).in_thread(thread_ts)).await
    }

    /// 向消息添加反应（emoji）
    ///
    /// 该方法用于向指定消息添加 emoji 反应。反应可用于表示
    /// 消息状态（如"已读"、"处理中"、"完成"等）。
    ///
    /// # 参数
    ///
    /// - `_channel_id`: 平台通道/会话标识符（如 Discord 的通道 ID）
    /// - `_message_id`: 平台范围的消息标识符（如 `discord_<snowflake>`）
    /// - `_emoji`: Unicode emoji 字符（如 "👀"、"✅"、"❌"）
    ///
    /// # 返回值
    ///
    /// - `Ok(())`: 添加成功或不支持（默认实现）
    /// - `Err(...)`: 添加失败
    ///
    /// # 平台支持
    ///
    /// - **Slack**: 支持 emoji 反应
    /// - **Discord**: 支持 emoji 反应
    /// - **Telegram**: 不支持（默认实现）
    ///
    /// # 示例
    ///
    /// ```ignore
    /// // 标记消息为已处理
    /// channel.add_reaction("channel_123", "msg_456", "✅").await?;
    /// ```
    async fn add_reaction(
        &self,
        _channel_id: &str,
        _message_id: &str,
        _emoji: &str,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    /// 从消息中移除之前由本机器人添加的反应（emoji）
    ///
    /// 该方法用于移除之前添加的 emoji 反应。可用于更新状态
    /// 或清理不再需要的反应。
    ///
    /// # 参数
    ///
    /// - `_channel_id`: 平台通道/会话标识符
    /// - `_message_id`: 平台范围的消息标识符
    /// - `_emoji`: 要移除的 Unicode emoji 字符
    ///
    /// # 返回值
    ///
    /// - `Ok(())`: 移除成功或不支持（默认实现）
    /// - `Err(...)`: 移除失败
    ///
    /// # 注意事项
    ///
    /// - 通常只能移除由机器人自己添加的反应
    /// - 某些平台可能有权限限制
    ///
    /// # 示例
    ///
    /// ```ignore
    /// // 更新状态：从"处理中"变为"完成"
    /// channel.remove_reaction("channel_123", "msg_456", "⏳").await?;
    /// channel.add_reaction("channel_123", "msg_456", "✅").await?;
    /// ```
    async fn remove_reaction(
        &self,
        _channel_id: &str,
        _message_id: &str,
        _emoji: &str,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
