//! 提供任务看板消息处理过程中复用的局部辅助逻辑。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use super::*;

/// TASK_BOARD_LOG_VIEWER_MAX_VISIBLE 使用的固定配置值。
pub(crate) const TASK_BOARD_LOG_VIEWER_MAX_VISIBLE: usize = 400;

/// 执行 format_task_log_timestamp 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn format_task_log_timestamp(timestamp_ms: u64) -> String {
    let total_seconds = timestamp_ms / 1000;
    let hh = (total_seconds / 3600) % 24;
    let mm = (total_seconds / 60) % 60;
    let ss = total_seconds % 60;
    format!("[{:02}:{:02}:{:02}] ", hh, mm, ss)
}

/// 执行 task_logs_viewer_text 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
#[cfg(test)]
pub(crate) fn task_logs_viewer_text(task: &Task) -> String {
    localized_task_logs_viewer_text(
        task,
        crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
    )
}

fn localized_task_logs_viewer_text(task: &Task, i18n: crate::app::i18n::I18n) -> String {
    task.logs
        .iter()
        .skip(task.logs.len().saturating_sub(TASK_BOARD_LOG_VIEWER_MAX_VISIBLE))
        .flat_map(|log| {
            let timestamp = format_task_log_timestamp(log.timestamp_ms);
            crate::app::i18n::tasks::log_message(i18n, log)
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(|line| format!("{timestamp}{line}"))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// 执行 sync_task_logs_editor 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn sync_task_logs_editor(app: &mut crate::app::App, task: Option<&Task>) {
    let next_text = task
        .map(|task| {
            localized_task_logs_viewer_text(task, crate::app::i18n::I18n::new(app.desktop_locale))
        })
        .unwrap_or_default();
    if app.task_board_logs_editor.text() == next_text.as_str() {
        if app.task_board_logs_auto_scroll {
            scroll_task_logs_to_bottom(app);
        }
        return;
    }

    app.task_board_logs_editor = if next_text.is_empty() {
        iced::widget::text_editor::Content::new()
    } else {
        iced::widget::text_editor::Content::with_text(&next_text)
    };

    app.task_board_logs_scroll_remainder = 0.0;

    if app.task_board_logs_auto_scroll {
        scroll_task_logs_to_bottom(app);
    } else {
        restore_task_logs_scroll(app, app.task_board_logs_scroll_top_line);
    }
}

/// 执行 task_logs_should_stay_in_memory 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn task_logs_should_stay_in_memory(status: TaskStatus) -> bool {
    matches!(
        status,
        TaskStatus::Planning
            | TaskStatus::Running
            | TaskStatus::CodeReview
            | TaskStatus::PrSubmitted
    )
}

/// 执行 merge_task_logs_with_cache 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn merge_task_logs_with_cache(
    logs: &[TaskLogEntry],
    cached_logs: Option<&[TaskLogEntry]>,
) -> Vec<TaskLogEntry> {
    let mut merged = logs.to_vec();

    if let Some(cache) = cached_logs {
        let overlap = logs
            .iter()
            .zip(cache.iter())
            .take_while(|(left, right)| {
                left.timestamp_ms == right.timestamp_ms && left.message == right.message
            })
            .count();
        let append_from = overlap.min(cache.len());
        merged.extend(cache[append_from..].iter().cloned());
    }

    merged
}

/// 执行 apply_cached_logs_to_task 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn apply_cached_logs_to_task(app: &mut crate::app::App, task: &mut Task) {
    let cached = app.task_board_log_cache.get(task.id.as_str()).map(Vec::as_slice);
    task.logs = merge_task_logs_with_cache(&task.logs, cached);
}

/// 执行 refresh_task_log_cache 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn refresh_task_log_cache(app: &mut crate::app::App, task: &Task) {
    if task_logs_should_stay_in_memory(task.status) {
        app.task_board_log_cache.insert(task.id.clone(), task.logs.clone());
    } else {
        app.task_board_log_cache.remove(task.id.as_str());
    }
}

/// 执行 refresh_task_log_cache_by_id 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn refresh_task_log_cache_by_id(app: &mut crate::app::App, task_id: &str) {
    if let Some(task) = app.task_board_tasks.iter().find(|task| task.id == task_id).cloned() {
        refresh_task_log_cache(app, &task);
    }
}

/// 执行 sync_task_log_cache_for_loaded_tasks 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn sync_task_log_cache_for_loaded_tasks(app: &mut crate::app::App) {
    let valid_ids = app
        .task_board_tasks
        .iter()
        .map(|task| task.id.clone())
        .collect::<std::collections::HashSet<_>>();
    app.task_board_log_cache.retain(|task_id, _| valid_ids.contains(task_id));

    let cache_snapshot = app.task_board_log_cache.clone();
    for task in &mut app.task_board_tasks {
        task.logs = merge_task_logs_with_cache(
            &task.logs,
            cache_snapshot.get(task.id.as_str()).map(Vec::as_slice),
        );
    }

    let tasks = app.task_board_tasks.clone();
    for task in &tasks {
        refresh_task_log_cache(app, task);
    }
}

/// 执行 set_viewing_logs 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn set_viewing_logs(app: &mut crate::app::App, task: Option<Task>) {
    let task = task.map(|mut task| {
        apply_cached_logs_to_task(app, &mut task);
        refresh_task_log_cache(app, &task);
        task
    });
    app.task_board_viewing_logs = task;
    let viewing_task = app.task_board_viewing_logs.clone();
    sync_task_logs_editor(app, viewing_task.as_ref());
}

/// 执行 sync_viewing_logs 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn sync_viewing_logs(app: &mut crate::app::App, task_id: &str) {
    if app.task_board_viewing_logs.as_ref().is_some_and(|viewing| viewing.id == task_id) {
        if let Some(updated) = app.task_board_tasks.iter().find(|t| t.id == task_id).cloned() {
            set_viewing_logs(app, Some(updated));
        }
    } else {
        refresh_task_log_cache_by_id(app, task_id);
    }
}

/// 执行 close_logs_context_menu 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn close_logs_context_menu(app: &mut crate::app::App) {
    app.task_board_logs_context_menu_open = false;
    app.task_board_logs_context_menu_pos = None;
}

/// 执行 task_logs_max_scroll_top_line 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn task_logs_max_scroll_top_line(app: &crate::app::App) -> f32 {
    let viewport_height = app.task_board_logs_viewport_height.max(1.0);
    let line_height = app.current_line_height.max(1.0);
    let total_lines = app.task_board_logs_editor.line_count().max(1) as f32;
    let visible_lines = (viewport_height / line_height).floor().max(1.0);
    (total_lines - visible_lines).max(0.0)
}

/// 执行 apply_task_logs_scroll_lines 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn apply_task_logs_scroll_lines(app: &mut crate::app::App, delta_lines: i32) {
    let max_scroll = task_logs_max_scroll_top_line(app);
    app.task_board_logs_scroll_top_line =
        (app.task_board_logs_scroll_top_line + delta_lines as f32).clamp(0.0, max_scroll);
    app.task_board_logs_auto_scroll = app.task_board_logs_scroll_top_line >= max_scroll;
}

/// 执行 restore_task_logs_scroll 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn restore_task_logs_scroll(app: &mut crate::app::App, target_top_line: f32) {
    let max_scroll = task_logs_max_scroll_top_line(app);
    let target_top_line = target_top_line.round().clamp(0.0, max_scroll);
    app.task_board_logs_scroll_top_line = target_top_line;

    let delta = target_top_line as i32;
    if delta != 0 {
        app.task_board_logs_editor
            .perform(iced::widget::text_editor::Action::Scroll { lines: delta });
    }

    app.task_board_logs_auto_scroll = target_top_line >= max_scroll;
}

/// 执行 scroll_task_logs_to_bottom 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn scroll_task_logs_to_bottom(app: &mut crate::app::App) {
    restore_task_logs_scroll(app, task_logs_max_scroll_top_line(app));
}
#[cfg(test)]
#[path = "logs_tests.rs"]
mod logs_tests;
