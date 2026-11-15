//! Telegram 频道流式草稿功能测试模块
//!
//! 本模块测试 Telegram 频道的流式消息草稿功能，包括：
//! - 流式模式的启用与配置
//! - 草稿消息的发送与更新
//! - 速率限制与网络优化
//! - UTF-8 多字节字符的安全截断
//! - 草稿定稿与回退机制

use super::*;
use channel_types::SendMessage;
use channel_types::Channel;
use crate::app::agent::config::StreamMode;

/// 测试 `supports_draft_updates` 方法是否正确响应流式模式配置
///
/// # 验证内容
/// - 当流式模式关闭时，应返回 `false`
/// - 当流式模式为 Partial 时，应返回 `true`
/// - 草稿更新间隔应正确设置
#[test]
fn supports_draft_updates_respects_stream_mode() {
    // 创建流式模式关闭的频道实例
    let off = TelegramChannel::new("fake-token".into(), vec!["*".into()], false);
    assert!(!off.supports_draft_updates());

    // 创建流式模式为 Partial 的频道实例，设置 750ms 更新间隔
    let partial = TelegramChannel::new("fake-token".into(), vec!["*".into()], false)
        .with_streaming(StreamMode::Partial, 750);
    assert!(partial.supports_draft_updates());
    assert_eq!(partial.draft_update_interval_ms, 750);
}

/// 测试当流式模式关闭时，`send_draft` 方法应返回 `None`
///
/// # 验证内容
/// - 流式模式关闭时，发送草稿消息应返回 `Ok(None)`
/// - 不应实际发送网络请求
#[tokio::test]
async fn send_draft_returns_none_when_stream_mode_off() {
    // 创建流式模式关闭的频道
    let ch = TelegramChannel::new("fake-token".into(), vec!["*".into()], false);

    // 尝试发送草稿消息
    let id = ch.send_draft(&SendMessage::new("draft", "123")).await.unwrap();

    // 验证返回值为 None
    assert!(id.is_none());
}

/// 测试草稿更新速率限制能够短路网络请求
///
/// # 验证内容
/// - 当距离上次草稿编辑时间过短时，应跳过网络请求
/// - 应返回 `Ok(())` 而不是错误
/// - 避免频繁调用 Telegram API
#[tokio::test]
async fn update_draft_rate_limit_short_circuits_network() {
    // 创建流式频道，设置较长的更新间隔（60秒）
    let ch = TelegramChannel::new("fake-token".into(), vec!["*".into()], false)
        .with_streaming(StreamMode::Partial, 60_000);

    // 模拟刚刚进行过草稿编辑（记录当前时间）
    ch.last_draft_edit.lock().insert("123".to_string(), std::time::Instant::now());

    // 尝试更新草稿，由于速率限制应跳过网络请求
    let result = ch.update_draft("123", "42", "delta text").await;

    // 验证操作成功（未发送网络请求）
    assert!(result.is_ok());
}

/// 测试 UTF-8 多字节字符文本的安全截断
///
/// # 验证内容
/// - 对于超长的 emoji 文本，截断操作不应导致 UTF-8 编码错误
/// - 应正确处理多字节字符边界
/// - 即使消息 ID 无效，也不应因编码问题而崩溃
#[tokio::test]
async fn update_draft_utf8_truncation_is_safe_for_multibyte_text() {
    // 创建流式频道，设置 0ms 更新间隔以绕过速率限制
    let ch = TelegramChannel::new("fake-token".into(), vec!["*".into()], false)
        .with_streaming(StreamMode::Partial, 0);

    // 构造超长的 emoji 文本（每个 emoji 占用 4 字节）
    let long_emoji_text = "😀".repeat(TELEGRAM_MAX_MESSAGE_LENGTH + 20);

    // 尝试更新草稿，即使消息 ID 无效也应安全处理
    let result = ch.update_draft("123", "not-a-number", &long_emoji_text).await;

    // 验证操作成功完成（UTF-8 截断安全）
    assert!(result.is_ok());
}

/// 测试草稿定稿时无效消息 ID 的回退机制
///
/// # 验证内容
/// - 当消息 ID 无效（非数字）时，应尝试分块发送
/// - 对于无法发送的超长消息，应返回错误
/// - 确保错误处理路径正常工作
#[tokio::test]
async fn finalize_draft_invalid_message_id_falls_back_to_chunk_send() {
    // 创建流式频道，设置 0ms 更新间隔
    let ch = TelegramChannel::new("fake-token".into(), vec!["*".into()], false)
        .with_streaming(StreamMode::Partial, 0);

    // 构造超长文本（超过 Telegram 消息长度限制）
    let long_text = "a".repeat(TELEGRAM_MAX_MESSAGE_LENGTH + 64);

    // 尝试定稿草稿，无效的消息 ID 会触发回退机制
    let result = ch.finalize_draft("123", "not-a-number", &long_text).await;

    // 由于消息过长且无法分块发送（测试环境），预期返回错误
    assert!(result.is_err());
}
