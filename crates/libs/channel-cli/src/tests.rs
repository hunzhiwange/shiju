//! CLI 通道模块的单元测试
//!
//! 本模块包含对 [`CliChannel`] 及相关数据结构的测试用例，覆盖以下功能：
//!
//! - 通道名称验证
//! - 消息发送行为
//! - 健康检查功能
//! - 消息结构体的字段访问与克隆能力
//!
//! ## 测试范围
//!
//! | 测试项 | 覆盖内容 |
//! |--------|----------|
//! | `cli_channel_name` | 通道名称返回值 |
//! | `cli_channel_send_does_not_panic` | 正常消息发送 |
//! | `cli_channel_send_empty_message` | 空消息发送 |
//! | `cli_channel_health_check` | 健康检查 |
//! | `channel_message_struct` | 消息结构体字段 |
//! | `channel_message_clone` | 消息克隆 |

use super::*;

/// CLI 通道测试模块
///
/// 包含所有针对 CLI 通道实现的单元测试。
/// 使用 `#[allow(dead_code)]` 属性是因为该模块仅在测试配置下编译。
#[allow(dead_code)]
mod tests {
    use super::*;

    #[cfg(not(target_arch = "wasm32"))]
    struct FailingReader;

    #[cfg(not(target_arch = "wasm32"))]
    impl tokio::io::AsyncRead for FailingReader {
        fn poll_read(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
            _buf: &mut tokio::io::ReadBuf<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            std::task::Poll::Ready(Err(std::io::Error::other("read failed")))
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    impl tokio::io::AsyncBufRead for FailingReader {
        fn poll_fill_buf(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<std::io::Result<&[u8]>> {
            std::task::Poll::Ready(Err(std::io::Error::other("read failed")))
        }

        fn consume(self: std::pin::Pin<&mut Self>, _amt: usize) {}
    }

    /// 测试 CLI 通道的名称返回值
    ///
    /// # 验证点
    ///
    /// - 新创建的 `CliChannel` 实例应返回 `"cli"` 作为其通道名称
    ///
    /// # 示例
    ///
    /// ```
    /// let channel = CliChannel::new();
    /// assert_eq!(channel.name(), "cli");
    /// ```
    #[test]
    fn cli_channel_name() {
        assert_eq!(CliChannel::new().name(), "cli");
    }

    /// 测试 CLI 通道发送消息不会触发 panic
    ///
    /// # 验证点
    ///
    /// - 发送包含正常内容的消息应成功完成
    /// - `send` 方法应返回 `Ok(())`
    ///
    /// # 输入数据
    ///
    /// - `content`: `"hello"`
    /// - `recipient`: `"user"`
    /// - `subject`: `None`
    /// - `thread_ts`: `None`
    #[tokio::test]
    async fn cli_channel_send_does_not_panic() {
        let ch = CliChannel::new();

        // 构造包含正常内容的发送消息请求
        let result = ch
            .send(&SendMessage {
                content: "hello".into(),
                recipient: "user".into(),
                subject: None,
                thread_ts: None,
            })
            .await;

        // 验证发送操作成功完成
        assert!(result.is_ok());
    }

    /// 测试 CLI 通道发送空消息的行为
    ///
    /// # 验证点
    ///
    /// - 发送空字符串内容应不会导致错误
    /// - CLI 通道应对边界情况（空内容、空接收者）保持健壮
    ///
    /// # 输入数据
    ///
    /// - `content`: 空字符串
    /// - `recipient`: 空字符串
    /// - `subject`: `None`
    /// - `thread_ts`: `None`
    #[tokio::test]
    async fn cli_channel_send_empty_message() {
        let ch = CliChannel::new();

        // 构造边界情况：所有字符串字段均为空
        let result = ch
            .send(&SendMessage {
                content: String::new(),
                recipient: String::new(),
                subject: None,
                thread_ts: None,
            })
            .await;

        // 验证即使是空消息也能成功发送
        assert!(result.is_ok());
    }

    /// 测试 CLI 通道的健康检查功能
    ///
    /// # 验证点
    ///
    /// - `health_check` 方法应始终返回 `true`
    /// - CLI 通道作为本地通道，不依赖外部服务，因此始终健康
    #[tokio::test]
    async fn cli_channel_health_check() {
        let ch = CliChannel::new();

        // CLI 通道是本地通道，健康检查应始终通过
        assert!(ch.health_check().await);
    }

    /// 测试 `ChannelMessage` 结构体的字段访问
    ///
    /// # 验证点
    ///
    /// - 所有字段在创建后可正确访问
    /// - 字段值与构造时传入的值一致
    ///
    /// # 测试数据
    ///
    /// | 字段 | 测试值 |
    /// |------|--------|
    /// | `id` | `"test-id"` |
    /// | `sender` | `"user"` |
    /// | `reply_target` | `"user"` |
    /// | `content` | `"hello"` |
    /// | `channel` | `"cli"` |
    /// | `timestamp` | `1234567890` |
    /// | `thread_ts` | `None` |
    #[test]
    fn channel_message_struct() {
        // 创建包含所有字段的消息实例
        let msg = ChannelMessage {
            instance_id: "default".to_string(),
            id: "test-id".into(),
            sender: "user".into(),
            reply_target: "user".into(),
            content: "hello".into(),
            channel: "cli".into(),
            timestamp: 1_234_567_890,
            thread_ts: None,
        };

        // 逐一验证各字段值
        assert_eq!(msg.id, "test-id");
        assert_eq!(msg.sender, "user");
        assert_eq!(msg.reply_target, "user");
        assert_eq!(msg.content, "hello");
        assert_eq!(msg.channel, "cli");
        assert_eq!(msg.timestamp, 1_234_567_890);
    }

    /// 测试 `ChannelMessage` 的克隆功能
    ///
    /// # 验证点
    ///
    /// - `ChannelMessage` 实现了 `Clone` trait
    /// - 克隆后的实例与原实例的字段值完全一致
    ///
    /// # 背景
    ///
    /// 消息克隆功能在需要将同一消息分发到多个处理器或进行消息副本操作时非常重要。
    #[test]
    fn channel_message_clone() {
        // 创建原始消息
        let msg = ChannelMessage {
            instance_id: "default".to_string(),
            id: "id".into(),
            sender: "s".into(),
            reply_target: "s".into(),
            content: "c".into(),
            channel: "ch".into(),
            timestamp: 0,
            thread_ts: None,
        };

        // 执行克隆操作
        let cloned = msg.clone();

        // 验证克隆后的关键字段与原始消息一致
        assert_eq!(cloned.id, msg.id);
        assert_eq!(cloned.content, msg.content);
    }

    #[test]
    fn cli_channel_is_zero_sized_and_constructible() {
        let channel = CliChannel::new();

        assert_eq!(std::mem::size_of_val(&channel), 0);
        assert_eq!(channel.name(), "cli");
    }

    #[test]
    fn unix_timestamp_secs_returns_elapsed_seconds() {
        let now = std::time::UNIX_EPOCH + std::time::Duration::from_secs(42);

        assert_eq!(unix_timestamp_secs(now), 42);
    }

    #[test]
    fn unix_timestamp_secs_returns_zero_before_epoch() {
        let now = std::time::UNIX_EPOCH - std::time::Duration::from_secs(1);

        assert_eq!(unix_timestamp_secs(now), 0);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn listen_from_reader_trims_skips_empty_and_stops_at_eof() {
        let (tx, mut rx) = tokio::sync::mpsc::channel(4);
        let reader = tokio::io::BufReader::new(std::io::Cursor::new(b"\n  hello  \n\t \n"));

        listen_from_reader(Box::new(reader), tx).await.unwrap();

        let msg = rx.recv().await.unwrap();

        assert!(!msg.id.is_empty());
        assert_eq!(msg.sender, "user");
        assert_eq!(msg.reply_target, "user");
        assert_eq!(msg.content, "hello");
        assert_eq!(msg.channel, "cli");
        assert!(msg.timestamp > 0);
        assert_eq!(msg.thread_ts, None);
        assert!(rx.recv().await.is_none());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn listen_from_reader_stops_on_quit_command() {
        let (tx, mut rx) = tokio::sync::mpsc::channel(4);
        let reader = tokio::io::BufReader::new(std::io::Cursor::new(b"/quit\nhello\n"));

        listen_from_reader(Box::new(reader), tx).await.unwrap();

        assert!(rx.recv().await.is_none());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn listen_from_reader_stops_on_exit_command() {
        let (tx, mut rx) = tokio::sync::mpsc::channel(4);
        let reader = tokio::io::BufReader::new(std::io::Cursor::new(b"/exit\nhello\n"));

        listen_from_reader(Box::new(reader), tx).await.unwrap();

        assert!(rx.recv().await.is_none());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn listen_from_reader_stops_when_receiver_is_closed() {
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let reader = tokio::io::BufReader::new(std::io::Cursor::new(b"hello\nignored\n"));
        drop(rx);

        listen_from_reader(Box::new(reader), tx).await.unwrap();
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn listen_from_reader_stops_on_read_error() {
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);

        listen_from_reader(Box::new(FailingReader), tx).await.unwrap();

        assert!(rx.recv().await.is_none());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn cli_channel_listen_reads_stdin_until_quit_command() {
        let (tx, mut rx) = tokio::sync::mpsc::channel(4);
        let reader =
            tokio::io::BufReader::new(std::io::Cursor::new(b"  hello  \n/quit\nignored\n"));

        listen_from_reader(Box::new(reader), tx).await.unwrap();

        let msg = rx.recv().await.unwrap();

        assert_eq!(msg.content, "hello");
        assert!(rx.recv().await.is_none());
    }
}
