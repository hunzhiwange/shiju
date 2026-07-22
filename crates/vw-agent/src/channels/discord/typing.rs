//! Discord 频道"正在输入"指示器管理模块
//!
//! 本模块实现了 Discord 消息通道的"正在输入"（typing indicator）功能。
//! 当机器人正在处理或准备发送消息时，可以通过周期性调用 Discord API
//! 来在用户界面显示"正在输入..."的状态提示。
//!
//! # 主要功能
//!
//! - 启动 typing 指示器：为指定频道/用户启动周期性的 typing 状态更新
//! - 停止 typing 指示器：在消息发送完成后停止 typing 状态
//! - 并发管理：支持多个频道的 typing 状态独立管理
//!
//! # 工作原理
//!
//! Discord 的 typing API 需要客户端每 8-10 秒发送一次 typing 请求，
//! 才能在用户界面持续显示"正在输入"状态。本模块通过异步任务循环
//! 实现这一机制，每个活跃的 typing 状态对应一个独立的异步任务。

use parking_lot::Mutex;
use std::collections::HashMap;

/// Typing 任务句柄映射表类型别名
///
/// 使用互斥锁保护的 HashMap，键为接收者标识（频道 ID 或用户 ID），
/// 值为对应的 typing 异步任务句柄。这种设计允许：
///
/// - 线程安全地管理多个并发 typing 任务
/// - 根据接收者 ID 快速查找和中止特定任务
/// - 防止同一接收者出现重复的 typing 任务
pub(super) type TypingHandles = Mutex<HashMap<String, tokio::task::JoinHandle<()>>>;

/// 创建新的 typing 句柄映射表
///
/// 初始化一个空的 typing handles 容器，用于存储和管理所有活跃的
/// typing 异步任务。每个 Discord 客户端实例应该维护一个独立的映射表。
///
/// # 返回值
///
/// 返回一个空的 `TypingHandles` 实例，包装在 Mutex 中以支持跨线程共享。
///
/// # 示例
///
/// ```rust,ignore
/// let handles = new_typing_handles();
/// // handles 现在可以安全地在多个线程间共享
/// ```
pub(super) fn new_typing_handles() -> TypingHandles {
    Mutex::new(HashMap::new())
}

/// 启动指定接收者的 typing 指示器
///
/// 为指定的 Discord 频道或用户启动一个周期性发送 typing 状态的异步任务。
/// 如果该接收者已有活跃的 typing 任务，会先停止旧任务再启动新任务。
///
/// # 工作机制
///
/// 1. 停止该接收者的任何现有 typing 任务（防止重复）
/// 2. 启动一个新的异步任务，该任务会：
///    - 构造 Discord typing API 端点 URL
///    - 循环发送 POST 请求到 typing 端点
///    - 每次请求后等待 8 秒（符合 Discord API 要求）
/// 3. 将新任务句柄存入映射表以便后续管理
///
/// # 参数
///
/// * `typing_handles` - typing 任务句柄映射表的引用，用于存储新任务
/// * `client` - HTTP 客户端，用于发送 Discord API 请求
/// * `bot_token` - Discord 机器人令牌，用于 API 认证
/// * `recipient` - 接收者标识（频道 ID 或用户 ID）
///
/// # 返回值
///
/// * `Ok(())` - typing 任务成功启动并已注册
/// * `Err(e)` - 启动失败（例如停止旧任务时出错）
///
/// # 示例
///
/// ```rust,ignore
/// let handles = new_typing_handles();
/// let client = reqwest::Client::new();
/// let token = "your_bot_token".to_string();
/// let channel_id = "1234567890".to_string();
///
/// start_typing(&handles, client, token, channel_id).await?;
/// // 现在用户会在该频道看到"正在输入..."提示
/// ```
///
/// # 注意事项
///
/// - typing 任务会无限循环直到被显式停止
/// - 务必在消息发送完成后调用 `stop_typing` 以释放资源
/// - Discord API 要求至少每 10 秒发送一次 typing 请求，本实现使用 8 秒间隔
pub(super) async fn start_typing(
    typing_handles: &TypingHandles,
    client: reqwest::Client,
    bot_token: String,
    api_base_url: String,
    recipient: String,
) -> anyhow::Result<()> {
    // 先停止该接收者的任何现有 typing 任务，避免重复
    stop_typing(typing_handles, &recipient).await?;

    // 克隆接收者 ID 用于异步任务（任务将获得所有权）
    let recipient_for_task = recipient.clone();

    // 启动异步任务：循环发送 typing 状态
    let handle = tokio::spawn(async move {
        // 构造 Discord typing API 端点 URL
        let url =
            format!("{}/channels/{recipient_for_task}/typing", api_base_url.trim_end_matches('/'));

        // 无限循环，每 8 秒发送一次 typing 请求
        loop {
            // 发送 POST 请求到 typing 端点
            // 使用 Bot 认证头格式：Bot {token}
            // 忽略响应结果（typing 请求的失败不应中断循环）
            let _ =
                client.post(&url).header("Authorization", format!("Bot {bot_token}")).send().await;

            // 等待 8 秒后继续下一次 typing 请求
            // Discord API 要求间隔不超过 10 秒，8 秒留有安全余量
            tokio::time::sleep(std::time::Duration::from_secs(8)).await;
        }
    });

    // 将任务句柄存入映射表，以便后续可以停止该任务
    let mut guard = typing_handles.lock();
    guard.insert(recipient, handle);

    Ok(())
}

/// 停止指定接收者的 typing 指示器
///
/// 中止指定接收者（频道/用户）的活跃 typing 任务，停止显示"正在输入"状态。
/// 如果该接收者没有活跃的 typing 任务，则此函数无操作。
///
/// # 参数
///
/// * `typing_handles` - typing 任务句柄映射表的引用
/// * `recipient` - 接收者标识（频道 ID 或用户 ID）
///
/// # 返回值
///
/// * `Ok(())` - 操作成功（无论是否存在活跃任务）
/// * `Err(e)` - 操作失败（极少见，主要来自锁操作）
///
/// # 示例
///
/// ```rust,ignore
/// let handles = new_typing_handles();
/// // ... 假设之前启动了 typing ...
///
/// // 消息发送完成后，停止 typing 指示器
/// stop_typing(&handles, "1234567890").await?;
/// ```
///
/// # 注意事项
///
/// - 此函数使用 `abort()` 强制终止任务，任务会被立即取消
/// - 对不存在的接收者调用此函数是安全的（无操作）
/// - 应在消息发送完成或需要提前取消 typing 时调用
pub(super) async fn stop_typing(
    typing_handles: &TypingHandles,
    recipient: &str,
) -> anyhow::Result<()> {
    // 获取映射表的互斥锁
    let mut guard = typing_handles.lock();

    // 尝试移除该接收者的任务句柄
    // 如果存在则中止该任务，否则无操作
    if let Some(handle) = guard.remove(recipient) {
        // 强制中止 typing 异步任务
        handle.abort();
    }

    Ok(())
}

#[cfg(test)]
#[path = "typing_tests.rs"]
mod typing_tests;
