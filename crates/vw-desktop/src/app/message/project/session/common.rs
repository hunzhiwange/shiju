//! 提供项目会话消息处理共享逻辑，封装配置保存和状态同步任务。
//!
//! 注释聚焦模块职责、消息边界和失败处理方式，帮助维护者在不改变逻辑的前提下理解代码。

#[cfg(not(target_arch = "wasm32"))]
use crate::app::config::set_config_field;
use crate::app::{App, Message};
use std::collections::HashMap;

/// parse_clamped_u32 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(super) fn parse_clamped_u32(raw: &str, fallback: u32, min: u32, max: u32) -> u32 {
    raw.trim().parse::<u32>().map(|v| v.clamp(min, max)).unwrap_or(fallback.clamp(min, max))
}

#[cfg(test)]
#[path = "common_tests.rs"]
mod common_tests;

/// parse_clamped_u64 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(super) fn parse_clamped_u64(raw: &str, fallback: u64, min: u64, max: u64) -> u64 {
    raw.trim().parse::<u64>().map(|v| v.clamp(min, max)).unwrap_or(fallback.clamp(min, max))
}

/// trim_to_option 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(super) fn trim_to_option(value: String) -> Option<String> {
    let value = value.trim().to_string();
    if value.is_empty() { None } else { Some(value) }
}

/// reset_new_session_picker_state 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(super) fn reset_new_session_picker_state(app: &mut App) {
    app.new_session_picker_project = None;
    app.new_session_picker_options.clear();
    app.new_session_worktree_name.clear();
    app.new_session_confirm_delete_directory = None;
    app.new_session_force_delete_directory = None;
    app.localized_input_errors.remove(&crate::app::i18n::input::ErrorField::WorktreeDelete);
    app.new_session_delete_error = None;
    app.new_session_confirm_reset_directory = None;
    app.localized_input_errors.remove(&crate::app::i18n::input::ErrorField::WorktreeReset);
    app.new_session_reset_error = None;
}

/// clear_new_session_picker_messages 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(super) fn clear_new_session_picker_messages(app: &mut App) {
    app.new_session_confirm_delete_directory = None;
    app.new_session_force_delete_directory = None;
    app.localized_input_errors.remove(&crate::app::i18n::input::ErrorField::WorktreeDelete);
    app.new_session_delete_error = None;
    app.new_session_confirm_reset_directory = None;
    app.localized_input_errors.remove(&crate::app::i18n::input::ErrorField::WorktreeReset);
    app.new_session_reset_error = None;
}

#[cfg(target_arch = "wasm32")]
/// save_config_field_task 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(super) fn save_config_field_task(
    key: &'static str,
    value: serde_json::Value,
) -> iced::Task<Message> {
    let key_owned = key.to_string();
    iced::Task::perform(
        async move {
            let mut cfg = crate::app::config::load_app_config_async().await?;
            if let Some(obj) = cfg.as_object_mut() {
                obj.insert(key_owned.clone(), value);
            } else {
                cfg = serde_json::json!({ key_owned: value });
            }
            crate::app::config::save_app_config_async(cfg).await
        },
        move |result| {
            if let Err(error) = result {
                tracing::warn!(target: "vw_desktop", key = key, error = %error, "failed to save desktop preference field");
            }
            Message::None
        },
    )
}

#[cfg(not(target_arch = "wasm32"))]
/// save_config_field_task 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(super) fn save_config_field_task(
    key: &'static str,
    value: serde_json::Value,
) -> iced::Task<Message> {
    set_config_field(key, value);
    iced::Task::none()
}

#[cfg(target_arch = "wasm32")]
/// save_project_worktree_enabled_task 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(super) fn save_project_worktree_enabled_task(
    project_worktree_enabled: HashMap<String, bool>,
) -> iced::Task<Message> {
    iced::Task::perform(
        async move {
            crate::app::config::update_system_settings_config_result_async(|system| {
                system.project_worktree_enabled = project_worktree_enabled;
            })
            .await
        },
        |result| {
            if let Err(error) = result {
                tracing::warn!(target: "vw_desktop", error = %error, "failed to save project worktree system settings");
            }
            Message::None
        },
    )
}

#[cfg(not(target_arch = "wasm32"))]
/// save_project_worktree_enabled_task 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(super) fn save_project_worktree_enabled_task(
    project_worktree_enabled: HashMap<String, bool>,
) -> iced::Task<Message> {
    crate::app::update_system_settings_config(|system| {
        system.project_worktree_enabled = project_worktree_enabled;
    });
    iced::Task::none()
}
