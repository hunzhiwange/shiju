//! 提供任务看板消息处理过程中复用的局部辅助逻辑。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use super::*;

/// 执行 save_settings 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn save_settings(app: &mut crate::app::App) {
    use crate::app::{
        projects::save_recent_projects_meta_background,
        state::{
            RecentProjectMeta, default_recent_project_session_auto_refresh,
            default_recent_project_session_refresh_interval_seconds,
        },
    };

    if let Some(path) = &app.project_path {
        let path = path.clone();
        if let Some(m) = app.recent_projects_meta.iter_mut().find(|m| m.path == path) {
            m.task_board_settings = Some(app.task_board_settings.clone());
        } else {
            let name = std::path::Path::new(&path)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(&path)
                .to_string();
            app.recent_projects_meta.push(RecentProjectMeta {
                path: path.clone(),
                name,
                task_board_settings: Some(app.task_board_settings.clone()),
                session_auto_refresh: default_recent_project_session_auto_refresh(),
                session_refresh_interval_seconds:
                    default_recent_project_session_refresh_interval_seconds(),
                icon: None,
                icon_color: None,
                worktree_start_command: None,
            });
        }
        save_recent_projects_meta_background(app.recent_projects_meta.clone());
    }
}

/// 执行 now_ms 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn now_ms() -> u64 {
    crate::app::time::now_ms()
}

/// 执行 next_deadline_ms 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn next_deadline_ms(interval_secs: u64) -> u64 {
    now_ms().saturating_add(interval_secs.saturating_mul(1000))
}

/// 执行 task_board_refresh_interval_secs 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn task_board_refresh_interval_secs(app: &crate::app::App) -> u64 {
    app.task_board_settings.refresh_interval_seconds.clamp(1, 3600)
}

/// 执行 sanitized_task_board_settings 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn sanitized_task_board_settings(settings: TaskBoardSettings) -> TaskBoardSettings {
    settings.sanitized()
}

/// 执行 scheduler_tick_interval_secs 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn scheduler_tick_interval_secs(app: &crate::app::App) -> u64 {
    app.task_board_settings.scheduler_tick_interval_seconds.clamp(1, 60)
}

/// 执行 auto_promote_tick_interval_secs 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn auto_promote_tick_interval_secs(app: &crate::app::App) -> u64 {
    app.task_board_settings.auto_promote_tick_interval_seconds.clamp(1, 3600)
}

/// 执行 pr_submitted_stall_timeout_secs 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
#[allow(dead_code)]
pub(crate) fn pr_submitted_stall_timeout_secs(app: &crate::app::App) -> u64 {
    app.task_board_settings.pr_submitted_stall_timeout_seconds.clamp(300, 3600) as u64
}

/// 执行 schedule_scheduler_tick 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn schedule_scheduler_tick(app: &crate::app::App) -> iced::Task<crate::app::Message> {
    let delay = std::time::Duration::from_secs(scheduler_tick_interval_secs(app));
    crate::app::message::after(
        delay,
        crate::app::Message::TaskBoard(TaskBoardMessage::ExecutionTick),
    )
}

/// 执行 schedule_auto_review_tick 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn schedule_auto_review_tick() -> iced::Task<crate::app::Message> {
    crate::app::message::after(
        std::time::Duration::from_secs(TASK_AUTO_CODE_REVIEW_TICK_INTERVAL_SECS),
        crate::app::Message::TaskBoard(TaskBoardMessage::AutoCodeReviewTick),
    )
}

/// 执行 schedule_auto_merge_tick 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn schedule_auto_merge_tick() -> iced::Task<crate::app::Message> {
    crate::app::message::after(
        std::time::Duration::from_secs(TASK_AUTO_MERGE_TICK_INTERVAL_SECS),
        crate::app::Message::TaskBoard(TaskBoardMessage::AutoMergeTick),
    )
}

/// 执行 schedule_scheduler_tick_with_deadline 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn schedule_scheduler_tick_with_deadline(
    app: &mut crate::app::App,
) -> iced::Task<crate::app::Message> {
    if app.task_board_next_scheduler_tick_at_ms <= now_ms() {
        app.task_board_next_scheduler_tick_at_ms =
            next_deadline_ms(scheduler_tick_interval_secs(app));
    }
    schedule_scheduler_tick(app)
}

/// 执行 schedule_auto_review_tick_with_deadline 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn schedule_auto_review_tick_with_deadline(
    app: &mut crate::app::App,
) -> iced::Task<crate::app::Message> {
    app.task_board_next_auto_review_tick_at_ms =
        next_deadline_ms(TASK_AUTO_CODE_REVIEW_TICK_INTERVAL_SECS);
    schedule_auto_review_tick()
}

/// 执行 schedule_auto_merge_tick_with_deadline 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn schedule_auto_merge_tick_with_deadline(
    app: &mut crate::app::App,
) -> iced::Task<crate::app::Message> {
    app.task_board_next_auto_merge_tick_at_ms =
        next_deadline_ms(TASK_AUTO_MERGE_TICK_INTERVAL_SECS);
    schedule_auto_merge_tick()
}

/// 执行 schedule_auto_promote_tick_with_deadline 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn schedule_auto_promote_tick_with_deadline(
    app: &mut crate::app::App,
) -> iced::Task<crate::app::Message> {
    let interval_secs = auto_promote_tick_interval_secs(app);
    app.task_board_next_auto_promote_tick_at_ms = next_deadline_ms(interval_secs);
    crate::app::message::after(
        std::time::Duration::from_secs(interval_secs),
        crate::app::Message::TaskBoard(TaskBoardMessage::PromotePoolTasksTick),
    )
}
#[cfg(test)]
#[path = "settings_tests.rs"]
mod settings_tests;
