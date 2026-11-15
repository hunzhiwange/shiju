//! 通道任务模式处理模块。
//!
//! 任务模式把发送者的后续普通消息转换为应用任务，而不是进入常规代理对话。
//! 本模块只维护发送者维度的开关与消息拦截逻辑，实际任务创建仍交给
//! 宿主的强类型任务投递操作，以保持通道层和任务存储/执行层之间的边界清晰。

use super::*;

/// 设置当前发送者是否启用任务模式。
///
/// # 参数
/// - `ctx`: 通道运行时上下文，用于读取和写回路由选择。
/// - `sender_key`: 当前发送者的稳定会话键。
/// - `enabled`: `true` 表示后续消息创建任务，`false` 表示恢复普通对话。
pub fn set_sender_task_mode(ctx: &ChannelRuntimeContext, sender_key: &str, enabled: bool) {
    let mut current = get_route_selection(ctx, sender_key);
    current.task_mode_enabled = enabled;
    set_route_selection(ctx, sender_key, current);
}

/// 在任务模式启用时拦截普通消息并创建任务。
///
/// # 参数
/// - `ctx`: 通道运行时上下文，用于读取发送者路由和工作区目录。
/// - `msg`: 当前收到的通道消息。
/// - `sender_key`: 当前发送者的稳定会话键。
/// - `target_channel`: 可选的回复通道；缺失时只消费消息，不发送反馈。
///
/// # 返回值
/// 返回 `true` 表示消息已被任务模式消费，调用方不应继续普通对话处理；
/// 返回 `false` 表示任务模式未启用。
///
/// # 错误处理
/// 任务创建错误会转换为用户可见的失败消息；发送反馈失败只记录警告，
/// 避免通道发送异常阻断消息消费状态。
pub async fn handle_task_mode_message_if_needed(
    host: &dyn crate::command_host::CommandHost,
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
    sender_key: &str,
    target_channel: Option<&Arc<dyn Channel>>,
) -> bool {
    let current = get_route_selection(ctx, sender_key);
    if !current.task_mode_enabled {
        return false;
    }
    let Some(channel) = target_channel else {
        // 没有可回复通道时仍返回已消费，避免同一条任务消息落回普通对话路径。
        return true;
    };
    let content = msg.content.trim();
    if content.is_empty() {
        // 空内容无法形成有效任务，直接消费可避免创建无意义任务记录。
        return true;
    }

    let model = (current.model != "auto").then_some(current.model.as_str());
    let project_dir = channel_project_directory(ctx);
    let project_path = project_dir.to_string_lossy().to_string();
    let response = match host.create_task(&project_path, 999, model, content) {
        Ok(created_task) => format!("任务已创建，任务编号：{}", created_task),
        Err(err) => format!("任务创建失败：{err}"),
    };

    if let Err(err) = channel
        .send(&SendMessage::new(response, &msg.reply_target).in_thread(msg.thread_ts.clone()))
        .await
    {
        tracing::warn!("Failed to send runtime command response on {}: {err}", channel.name());
    }
    true
}
