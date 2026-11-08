//! 提供项目会话消息的输入解析与选择器状态清理。
//!
//! 注释聚焦模块职责、消息边界和失败处理方式，帮助维护者在不改变逻辑的前提下理解代码。

use crate::app::App;

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
