//! 心跳设置消息处理模块
//!
//! 本模块负责处理与心跳功能相关的所有设置消息，包括：
//! - 启用/禁用心跳功能
//! - 配置心跳间隔时间
//! - 设置心跳消息内容、目标和接收者
//! - 管理帮助模态框的显示状态
//!
//! 所有设置变更都会自动持久化到配置文件中，确保应用重启后配置不丢失。

use super::messages::Message;
use desktop_config::ConfigAccess;
use iced::Task;

use super::messages::SettingsMessage;

/// 持久化心跳设置到配置文件
///
/// 该函数从领域状态中读取当前的心跳设置，并将其写入配置文件。
/// 主要处理逻辑：
/// 1. 从领域状态中提取心跳设置数据
/// 2. 对输入字段进行去除首尾空格处理
/// 3. 将空字符串转换为 None，非空字符串包装为 Some
/// 4. 确保心跳间隔在有效范围内（1-1440分钟，即1分钟到24小时）
/// 5. 调用全局配置更新函数写入配置文件
///
/// # 参数
///
/// * `app` - 领域输入，从中读取心跳设置状态
fn persist_heartbeat_settings(app: &mut Inputs<'_>) -> Task<Message> {
    // 从领域状态中获取心跳设置的引用
    let s = &app.heartbeat_settings;

    // 提取并清理各个输入字段，去除首尾空格
    let enabled = s.enabled;
    let interval_minutes = s.interval_minutes.clamp(1, 1440);
    let message = s.message_input.trim().to_string();
    let target = s.target_input.trim().to_string();
    let to = s.to_input.trim().to_string();

    // 更新全局心跳配置文件
    // 使用闭包模式更新配置，避免直接操作配置文件
    app.config.update_heartbeat_config_async(move |heartbeat| {
        heartbeat.enabled = enabled;

        // 设置心跳间隔，限制在1-1440分钟范围内（1分钟到24小时）
        heartbeat.interval_minutes = interval_minutes;

        // 设置心跳消息内容，空字符串转换为 None
        heartbeat.message = if message.is_empty() { None } else { Some(message) };

        // 设置心跳目标，空字符串转换为 None
        heartbeat.target = if target.is_empty() { None } else { Some(target) };

        // 设置消息接收者，空字符串转换为 None
        heartbeat.to = if to.is_empty() { None } else { Some(to) };
    }).map(Message::ConfigSaved)
}



/// 处理心跳设置相关的消息更新
///
/// 该函数是心跳设置模块的核心消息处理器，负责响应各种心跳设置相关的用户操作。
/// 所有设置变更都会自动持久化到配置文件，并清除之前的保存错误信息。
///
/// # 参数
///
/// * `app` - 领域输入，用于更新领域状态
/// * `message` - 设置消息枚举，表示具体的用户操作
///
/// # 返回值
///
/// 返回 `Task<Message>`，可能包含需要执行的异步任务。大多数情况下返回 `Task::none()`
/// 表示无需执行额外任务。
///
/// # 消息类型处理
///
/// - `HeartbeatEnabledToggled` - 切换心跳功能的启用/禁用状态
/// - `HeartbeatIntervalChanged` - 修改心跳间隔时间（自动限制在1-1440分钟范围内）
/// - `HeartbeatMessageChanged` - 修改心跳消息内容
/// - `HeartbeatTargetChanged` - 修改心跳目标
/// - `HeartbeatToChanged` - 修改消息接收者
/// - `HeartbeatSave` - 手动保存当前设置
/// - `HeartbeatHelpOpen` - 打开帮助模态框
/// - `HeartbeatHelpClose` - 关闭帮助模态框
/// - 其他消息 - 返回空任务（不处理）
pub fn update(app: &mut Inputs<'_>, message: SettingsMessage) -> Task<Message> {
    match message {
        // 处理心跳功能启用状态切换
        SettingsMessage::HeartbeatEnabledToggled(v) => {
            // 更新启用状态
            app.heartbeat_settings.enabled = v;
            persist_heartbeat_settings(app)
        }

        // 处理心跳间隔变更
        SettingsMessage::HeartbeatIntervalChanged(v) => {
            // 将间隔限制在有效范围内（1-1440分钟）并更新状态
            app.heartbeat_settings.interval_minutes = v.clamp(1, 1440);
            persist_heartbeat_settings(app)
        }

        // 处理心跳消息内容变更
        SettingsMessage::HeartbeatMessageChanged(v) => {
            // 更新消息输入框内容
            app.heartbeat_settings.message_input = v;
            persist_heartbeat_settings(app)
        }

        // 处理心跳目标变更
        SettingsMessage::HeartbeatTargetChanged(v) => {
            // 更新目标输入框内容
            app.heartbeat_settings.target_input = v;
            persist_heartbeat_settings(app)
        }

        // 处理消息接收者变更
        SettingsMessage::HeartbeatToChanged(v) => {
            // 更新接收者输入框内容
            app.heartbeat_settings.to_input = v;
            persist_heartbeat_settings(app)
        }

        // 处理手动保存请求
        SettingsMessage::HeartbeatSave => persist_heartbeat_settings(app),

        // 处理打开帮助模态框
        SettingsMessage::HeartbeatHelpOpen => {
            // 显示帮助模态框
            app.heartbeat_settings.show_help_modal = true;
            Task::none()
        }

        // 处理关闭帮助模态框
        SettingsMessage::HeartbeatHelpClose => {
            // 隐藏帮助模态框
            app.heartbeat_settings.show_help_modal = false;
            Task::none()
        }

        // 处理其他不相关的消息，返回空任务
        _ => Task::none(),
    }
}

pub struct Inputs<'a> {
pub heartbeat_settings: &'a mut crate::app_init::state::HeartbeatSettingsState,
 pub config: &'a ConfigAccess,
}

#[cfg(test)] #[path = "heartbeat_tests.rs"] mod tests;
