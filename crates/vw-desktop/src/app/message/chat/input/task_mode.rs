//! 处理聊天输入区的局部消息。
//! 本模块将编辑器操作、文件检索和工具细节限制在输入面板边界内。

use crate::app::{App, Message};
use iced::{Task, widget::text_editor};

/// 模块内可见函数，执行 handle_task_mode_toggled 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_task_mode_toggled(app: &mut App, enabled: bool) -> Task<Message> {
    let runtime = app.current_session_runtime_mut();
    runtime.task_mode_enabled = enabled;
    Task::none()
}

/// 模块内可见函数，执行 handle_workflow_mode_toggled 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_workflow_mode_toggled(app: &mut App, enabled: bool) -> Task<Message> {
    app.current_session_runtime_mut().workflow_mode_enabled = enabled;
    Task::none()
}

/// 切换目标模式；可与工作流并存，互不关闭。
pub(super) fn handle_goal_mode_toggled(app: &mut App, enabled: bool) -> Task<Message> {
    app.current_session_runtime_mut().goal_mode_enabled = enabled;
    Task::none()
}

/// 切换子智能体模式；该模式独立于工作流与目标模式。
pub(super) fn handle_subagent_mode_toggled(app: &mut App, enabled: bool) -> Task<Message> {
    app.current_session_runtime_mut().subagent_mode_enabled = enabled;
    Task::none()
}

/// 模块内可见函数，执行 handle_task_mode_priority_changed 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_task_mode_priority_changed(app: &mut App, value: String) -> Task<Message> {
    let runtime = app.current_session_runtime_mut();
    runtime.task_mode_priority = value;
    Task::none()
}

/// 模块内可见函数，执行 handle_task_mode_model_changed 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_task_mode_model_changed(app: &mut App, model: String) -> Task<Message> {
    let runtime = app.current_session_runtime_mut();
    let trimmed = model.trim();
    runtime.task_mode_model =
        if trimmed.is_empty() { "auto".to_string() } else { trimmed.to_string() };
    Task::none()
}

/// 模块内可见函数，执行 handle_task_mode_task_split_mode_changed 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_task_mode_task_split_mode_changed(
    app: &mut App,
    mode: crate::app::task::TaskSplitMode,
) -> Task<Message> {
    let runtime = app.current_session_runtime_mut();
    runtime.task_mode_task_split_mode = mode;
    Task::none()
}

/// 模块内可见函数，执行 handle_task_mode_executor_changed 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_task_mode_executor_changed(
    app: &mut App,
    executor: Option<String>,
) -> Task<Message> {
    let runtime = app.current_session_runtime_mut();
    runtime.task_mode_executor = executor;
    Task::none()
}

pub(super) fn handle_task_mode_reasoning_effort_changed(
    app: &mut App,
    reasoning_effort: Option<vw_gateway_client::vw_api_types::ReasoningEffortDto>,
) -> Task<Message> {
    app.current_session_runtime_mut().task_mode_reasoning_effort = reasoning_effort;
    Task::none()
}

/// 模块内可见函数，执行 handle_task_mode_subtask_changed 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_task_mode_subtask_changed(
    app: &mut App,
    index: usize,
    value: String,
) -> Task<Message> {
    let runtime = app.current_session_runtime_mut();
    if let Some(item) = runtime.task_mode_subtasks.get_mut(index) {
        *item = value.clone();
    }
    if let Some(editor) = runtime.task_mode_subtask_editors.get_mut(index) {
        *editor = text_editor::Content::with_text(&value);
        editor.perform(text_editor::Action::Move(text_editor::Motion::DocumentEnd));
    }
    Task::none()
}

/// 模块内可见函数，执行 handle_task_mode_subtask_editor_action 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_task_mode_subtask_editor_action(
    app: &mut App,
    index: usize,
    action: text_editor::Action,
) -> Task<Message> {
    let runtime = app.current_session_runtime_mut();
    if index >= runtime.task_mode_subtasks.len() {
        return Task::none();
    }

    while runtime.task_mode_subtask_editors.len() < runtime.task_mode_subtasks.len() {
        runtime.task_mode_subtask_editors.push(text_editor::Content::new());
    }

    if let Some(editor) = runtime.task_mode_subtask_editors.get_mut(index) {
        let editor: &mut text_editor::Content = editor;
        let action: text_editor::Action = action;
        editor.perform(action);
        let value = editor.text().to_string();
        if let Some(item) = runtime.task_mode_subtasks.get_mut(index) {
            *item = value;
        }
    }

    Task::none()
}

/// 模块内可见函数，执行 handle_task_mode_add_subtask 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_task_mode_add_subtask(app: &mut App) -> Task<Message> {
    let runtime = app.current_session_runtime_mut();
    runtime.task_mode_subtasks.push(String::new());
    runtime.task_mode_subtask_editors.push(text_editor::Content::new());
    Task::none()
}

/// 模块内可见函数，执行 handle_task_mode_remove_subtask 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_task_mode_remove_subtask(app: &mut App, index: usize) -> Task<Message> {
    let runtime = app.current_session_runtime_mut();
    if index < runtime.task_mode_subtasks.len() {
        runtime.task_mode_subtasks.remove(index);
        if index < runtime.task_mode_subtask_editors.len() {
            runtime.task_mode_subtask_editors.remove(index);
        }
    }

    if runtime.task_mode_subtasks.is_empty() {
        runtime.task_mode_subtasks.push(String::new());
        runtime.task_mode_subtask_editors.push(text_editor::Content::new());
    }

    Task::none()
}

/// 模块内可见函数，执行 handle_task_mode_move_subtask_up 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_task_mode_move_subtask_up(app: &mut App, index: usize) -> Task<Message> {
    let runtime = app.current_session_runtime_mut();
    if index > 0 && index < runtime.task_mode_subtasks.len() {
        runtime.task_mode_subtasks.swap(index - 1, index);
        if index < runtime.task_mode_subtask_editors.len() {
            runtime.task_mode_subtask_editors.swap(index - 1, index);
        }
    }
    Task::none()
}

/// 模块内可见函数，执行 handle_task_mode_move_subtask_down 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn handle_task_mode_move_subtask_down(app: &mut App, index: usize) -> Task<Message> {
    let runtime = app.current_session_runtime_mut();
    if index + 1 < runtime.task_mode_subtasks.len() {
        runtime.task_mode_subtasks.swap(index, index + 1);
        if index + 1 < runtime.task_mode_subtask_editors.len() {
            runtime.task_mode_subtask_editors.swap(index, index + 1);
        }
    }
    Task::none()
}
#[cfg(test)]
#[path = "task_mode_tests.rs"]
mod task_mode_tests;
