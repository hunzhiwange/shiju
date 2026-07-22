//! CLI 通道模块
//!
//! 本模块提供基于标准输入/输出（stdin/stdout）的命令行界面通道实现。
//! 作为最基础的通信通道，CLI 通道具有以下特点：
//!
//! - **零外部依赖**：仅使用标准输入输出，无需额外配置
//! - **始终可用**：不依赖网络连接或外部服务
//! - **跨平台兼容**：支持原生平台和 WebAssembly 目标
//! - **交互式**：支持实时用户输入与响应
//!
//! # 架构位置
//!
//! CLI 通道位于 `channels` 模块下，实现了 [`Channel`] trait，
//! 是 VibeWindow 代理系统的核心通信通道之一。
//!
//! # 使用示例
//!
//! ```rust,no_run
//! use vibewindow::channels::{CliChannel, Channel, ChannelMessage};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // 创建 CLI 通道实例
//!     let channel = CliChannel::new();
//!
//!     // 获取通道名称
//!     println!("通道名称: {}", channel.name());
//!
//!     // 发送消息到标准输出
//!     use vibewindow::channels::SendMessage;
//!     let msg = SendMessage {
//!         content: "Hello, World!".to_string(),
//!     };
//!     channel.send(&msg).await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! # 特殊命令
//!
//! CLI 通道支持以下内置命令：
//! - `/quit` - 退出监听循环
//! - `/exit` - 退出监听循环
//!
//! # 平台差异
//!
//! - **原生平台**（非 WASM）：完整支持异步标准输入读取
//! - **WebAssembly**：监听操作将无限等待，因为 WASM 环境不支持直接访问 stdin

use super::traits::{Channel, ChannelMessage, SendMessage};
#[cfg(target_arch = "wasm32")]
use async_trait::async_trait;
#[cfg(not(target_arch = "wasm32"))]
use tokio::io::{self, AsyncBufReadExt, BufReader};
#[cfg(not(target_arch = "wasm32"))]
use uuid::Uuid;

/// CLI 通道结构体
///
/// 基于标准输入/输出的命令行界面通道实现。
/// 该通道直接与用户终端交互，是调试和基本交互的首选方式。
///
/// # 特性
///
/// - 无需认证或配置
/// - 同步阻塞式用户输入
/// - 异步非阻塞式消息发送
/// - 支持多行文本输入
///
/// # 线程安全
///
/// `CliChannel` 是零大小类型（ZST），可以安全地在多个线程间共享和克隆。
/// 所有状态都通过标准输入/输出句柄管理，这些句柄本身是线程安全的。
///
/// # 示例
///
/// ```rust
/// use vibewindow::channels::CliChannel;
///
/// let channel = CliChannel::new();
/// assert_eq!(channel.name(), "cli");
/// ```
pub struct CliChannel;

impl CliChannel {
    /// 创建新的 CLI 通道实例
    ///
    /// # 返回值
    ///
    /// 返回一个全新的 [`CliChannel`] 实例。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use vibewindow::channels::CliChannel;
    ///
    /// let channel = CliChannel::new();
    /// ```
    pub fn new() -> Self {
        Self
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn listen_from_reader(
    reader: Box<dyn tokio::io::AsyncBufRead + Unpin + Send>,
    tx: tokio::sync::mpsc::Sender<ChannelMessage>,
) -> anyhow::Result<()> {
    let mut lines = reader.lines();

    while let Ok(Some(line)) = lines.next_line().await {
        let line = line.trim().to_string();

        if line.is_empty() {
            continue;
        }

        if line == "/quit" || line == "/exit" {
            break;
        }

        let msg = ChannelMessage {
            id: Uuid::new_v4().to_string(),
            sender: "user".to_string(),
            reply_target: "user".to_string(),
            content: line,
            channel: "cli".to_string(),
            timestamp: unix_timestamp_secs(std::time::SystemTime::now()),
            thread_ts: None,
        };

        if tx.send(msg).await.is_err() {
            break;
        }
    }

    Ok(())
}

fn write_cli_message(message: &SendMessage) {
    use std::io::Write;

    let mut stdout = std::io::stdout();
    stdout.write_all(message.content.as_bytes()).expect("failed to write cli message");
    stdout.write_all(b"\n").expect("failed to write cli newline");
}

fn unix_timestamp_secs(now: std::time::SystemTime) -> u64 {
    now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()
}

/// 为 [`CliChannel`] 实现 [`Channel`] trait
///
/// 该实现提供了 CLI 通道的核心功能：
/// - 消息发送到标准输出
/// - 从标准输入监听用户消息
/// - 通道标识与管理
#[cfg(not(target_arch = "wasm32"))]
impl Channel for CliChannel {
    /// 获取通道名称
    ///
    /// # 返回值
    ///
    /// 返回静态字符串 `"cli"`，用于标识此通道类型。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use vibewindow::channels::{CliChannel, Channel};
    ///
    /// let channel = CliChannel::new();
    /// assert_eq!(channel.name(), "cli");
    /// ```
    fn name(&self) -> &str {
        "cli"
    }

    /// 发送消息到标准输出
    ///
    /// 将消息内容直接打印到终端标准输出。
    ///
    /// # 参数
    ///
    /// - `message`: 要发送的消息引用，包含消息内容
    ///
    /// # 返回值
    ///
    /// - `Ok(())`: 消息成功发送
    /// - `Err(...)`: 输出错误（极少发生）
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use vibewindow::channels::{CliChannel, Channel, SendMessage};
    ///
    /// # #[tokio::main]
    /// # async fn main() -> anyhow::Result<()> {
    /// let channel = CliChannel::new();
    /// let msg = SendMessage {
    ///     content: "Hello, World!".to_string(),
    /// };
    /// channel.send(&msg).await?;
    /// # Ok(())
    /// # }
    /// ```
    fn send<'life0, 'life1, 'async_trait>(
        &'life0 self,
        message: &'life1 SendMessage,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = anyhow::Result<()>> + Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            write_cli_message(message);
            Ok(())
        })
    }

    /// 监听标准输入并转发消息
    ///
    /// 启动异步监听循环，从标准输入读取用户输入并转换为通道消息。
    ///
    /// # 参数
    ///
    /// - `tx`: 多生产者单消费者通道的发送端，用于将用户输入转发给代理系统
    ///
    /// # 返回值
    ///
    /// - `Ok(())`: 监听正常结束（用户输入退出命令或通道关闭）
    /// - `Err(...)`: 读取错误或通道发送错误
    ///
    /// # 行为说明
    ///
    /// 1. **非 WASM 平台**：
    ///    - 异步读取标准输入的每一行
    ///    - 自动去除首尾空白字符
    ///    - 跳过空行
    ///    - 检测 `/quit` 或 `/exit` 命令并退出
    ///    - 为每条有效输入生成唯一的消息 ID
    ///    - 设置发送者为 `"user"`，回复目标为 `"user"`
    ///    - 记录当前 Unix 时间戳
    ///
    /// 2. **WASM 平台**：
    ///    - 调用 `std::future::pending()` 无限等待
    ///    - WASM 环境不支持直接访问标准输入
    ///
    /// # 退出条件
    ///
    /// 监听循环在以下情况下退出：
    /// - 用户输入 `/quit` 或 `/exit` 命令
    /// - 消息通道发送端关闭（接收端被丢弃）
    /// - 标准输入到达 EOF（文件结束）
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use tokio::sync::mpsc;
    /// use vibewindow::channels::{CliChannel, Channel, ChannelMessage};
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let (tx, mut rx) = mpsc::channel::<ChannelMessage>(100);
    ///     let channel = CliChannel::new();
    ///
    ///     // 在后台任务中运行监听
    ///     let handle = tokio::spawn(async move {
    ///         channel.listen(tx).await
    ///     });
    ///
    ///     // 处理接收到的消息
    ///     while let Some(msg) = rx.recv().await {
    ///         println!("收到消息: {}", msg.content);
    ///     }
    ///
    ///     handle.await??;
    ///     Ok(())
    /// }
    /// ```
    fn listen<'life0, 'async_trait>(
        &'life0 self,
        tx: tokio::sync::mpsc::Sender<ChannelMessage>,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = anyhow::Result<()>> + Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            let stdin = io::stdin();
            let reader = Box::new(BufReader::new(stdin));
            listen_from_reader(reader, tx).await
        })
    }
}

#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
impl Channel for CliChannel {
    fn name(&self) -> &str {
        "cli"
    }

    async fn send(&self, message: &SendMessage) -> anyhow::Result<()> {
        write_cli_message(message);
        Ok(())
    }

    async fn listen(&self, _tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> anyhow::Result<()> {
        std::future::pending::<()>().await;
        Ok(())
    }
}

/// 单元测试模块
///
/// 测试 CLI 通道的基本功能和边界条件。
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
