//! 渲染应用视图中的模态窗口。
//! 本模块只描述模态内容和交互消息，不直接承担持久化策略。

use crate::app::components::system_settings_common::{
    rounded_action_btn_style, settings_checkbox_style, settings_field_label,
    settings_group_heading, settings_muted_text_style, settings_panel, settings_select,
};
use crate::app::i18n::I18n;
use crate::app::message::ProjectMessage;
use iced::widget::{button, checkbox, column, row, text};
use iced::{Element, Length};

use super::common::numeric_row;
use super::{App, Message};

/// 工作流选择项，承载稳定 id 与展示标签，仅用于调度页内置工作流选择控件。
#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkflowOption {
    id: String,
    label: String,
}

impl std::fmt::Display for WorkflowOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

/// 模块内可见函数，执行 scheduling_tab 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn scheduling_tab<'a>(app: &'a App) -> Element<'a, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let settings = &app.project_edit_task_board_settings;
    let max_concurrent = settings.max_concurrent.clamp(1, 10);
    let scheduler_tick_interval_seconds = settings.scheduler_tick_interval_seconds.clamp(1, 60);
    let auto_promote_tick_interval_seconds =
        settings.auto_promote_tick_interval_seconds.clamp(1, 3600);
    let failed_retry_minutes = settings.failed_retry_minutes.clamp(1, 1440);
    let failed_retry_max_attempts = settings.failed_retry_max_attempts.clamp(0, 10);
    let running_timeout_minutes = settings.running_timeout_minutes.clamp(1, 1440);
    let pr_submitted_stall_timeout_seconds =
        settings.pr_submitted_stall_timeout_seconds.clamp(5, 3600);

    let workflow_options: Vec<WorkflowOption> = crate::app::task::all_task_workflows()
        .into_iter()
        .map(|workflow| WorkflowOption { id: workflow.id, label: workflow.label })
        .collect();
    let selected_workflow = workflow_options
        .iter()
        .find(|option| option.id == settings.workflow_id)
        .cloned()
        .or_else(|| workflow_options.first().cloned());
    let mut workflow_controls = column![
        settings_field_label(i18n.__("任务工作流")),
        row![
            settings_select(workflow_options, selected_workflow, |option| {
                Message::Project(ProjectMessage::ProjectEditWorkflowIdChanged(option.id))
            })
            .width(Length::Fill),
            button(text(i18n.__("管理工作流")).size(13))
                .on_press(Message::View(
                    crate::app::message::view::ViewMessage::OpenWorkflowManager
                ))
                .padding([9, 12])
                .style(rounded_action_btn_style),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center),
    ]
    .spacing(8);
    if let Some(hint) = workflow_cache_hint(app) {
        workflow_controls =
            workflow_controls.push(text(hint).size(13).style(settings_muted_text_style));
    }

    let max_concurrent_row =
        numeric_row(
            i18n.__("并发任务数"),
            "1-10",
            &app.project_edit_max_concurrent_input,
            |value| Message::Project(ProjectMessage::ProjectEditMaxConcurrentInputChanged(value)),
            Message::Project(ProjectMessage::ProjectEditMaxConcurrentChanged(
                if max_concurrent <= 1 { 1 } else { max_concurrent - 1 },
            )),
            Message::Project(ProjectMessage::ProjectEditMaxConcurrentChanged(
                if max_concurrent >= 10 { 10 } else { max_concurrent + 1 },
            )),
        );

    let scheduler_tick_interval_row = numeric_row(
        i18n.__("执行唤醒间隔(秒)"),
        "1-60",
        &app.project_edit_task_board_scheduler_tick_interval_seconds_input,
        |value| {
            Message::Project(
                ProjectMessage::ProjectEditTaskBoardSchedulerTickIntervalSecondsInputChanged(value),
            )
        },
        Message::Project(ProjectMessage::ProjectEditTaskBoardSchedulerTickIntervalSecondsChanged(
            scheduler_tick_interval_seconds.saturating_sub(1).max(1),
        )),
        Message::Project(ProjectMessage::ProjectEditTaskBoardSchedulerTickIntervalSecondsChanged(
            scheduler_tick_interval_seconds.saturating_add(1).min(60),
        )),
    );

    let auto_promote_tick_interval_row = numeric_row(
        i18n.__("任务池回推间隔(秒)"),
        "1-3600",
        &app.project_edit_task_board_auto_promote_tick_interval_seconds_input,
        |value| {
            Message::Project(
                ProjectMessage::ProjectEditTaskBoardAutoPromoteTickIntervalSecondsInputChanged(
                    value,
                ),
            )
        },
        Message::Project(
            ProjectMessage::ProjectEditTaskBoardAutoPromoteTickIntervalSecondsChanged(
                auto_promote_tick_interval_seconds.saturating_sub(5).max(1),
            ),
        ),
        Message::Project(
            ProjectMessage::ProjectEditTaskBoardAutoPromoteTickIntervalSecondsChanged(
                auto_promote_tick_interval_seconds.saturating_add(5).min(3600),
            ),
        ),
    );

    let failed_retry_row = numeric_row(
        i18n.__("失败重试(分钟)"),
        "1-1440",
        &app.project_edit_failed_retry_minutes_input,
        |value| Message::Project(ProjectMessage::ProjectEditFailedRetryMinutesInputChanged(value)),
        Message::Project(ProjectMessage::ProjectEditFailedRetryMinutesChanged(
            if failed_retry_minutes <= 1 { 1 } else { failed_retry_minutes - 1 },
        )),
        Message::Project(ProjectMessage::ProjectEditFailedRetryMinutesChanged(
            if failed_retry_minutes >= 1440 { 1440 } else { failed_retry_minutes + 1 },
        )),
    );

    let failed_retry_max_attempts_row = numeric_row(
        i18n.__("最大重试(次)"),
        "0-10",
        &app.project_edit_failed_retry_max_attempts_input,
        |value| {
            Message::Project(ProjectMessage::ProjectEditFailedRetryMaxAttemptsInputChanged(value))
        },
        Message::Project(ProjectMessage::ProjectEditFailedRetryMaxAttemptsChanged(
            failed_retry_max_attempts.saturating_sub(1),
        )),
        Message::Project(ProjectMessage::ProjectEditFailedRetryMaxAttemptsChanged(
            failed_retry_max_attempts.saturating_add(1).min(10),
        )),
    );

    let running_timeout_row = numeric_row(
        i18n.__("运行超时(分钟)"),
        "1-1440",
        &app.project_edit_running_timeout_minutes_input,
        |value| {
            Message::Project(ProjectMessage::ProjectEditRunningTimeoutMinutesInputChanged(value))
        },
        Message::Project(ProjectMessage::ProjectEditRunningTimeoutMinutesChanged(
            if running_timeout_minutes <= 1 { 1 } else { running_timeout_minutes - 1 },
        )),
        Message::Project(ProjectMessage::ProjectEditRunningTimeoutMinutesChanged(
            if running_timeout_minutes >= 1440 { 1440 } else { running_timeout_minutes + 1 },
        )),
    );

    let recycle_worktree_on_finish_row = row![
        settings_field_label(i18n.__("完成后回收 worktree")).width(Length::Fill),
        checkbox(settings.recycle_worktree_on_task_finish)
            .on_toggle(|enabled| {
                Message::Project(ProjectMessage::ProjectEditRecycleWorktreeOnTaskFinishToggled(
                    enabled,
                ))
            })
            .style(settings_checkbox_style),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center);

    let pr_stall_timeout_row = numeric_row(
        i18n.__("合并锁超时(秒)"),
        "5-3600",
        &app.project_edit_pr_submitted_stall_timeout_seconds_input,
        |value| {
            Message::Project(ProjectMessage::ProjectEditPrSubmittedStallTimeoutSecondsInputChanged(
                value,
            ))
        },
        Message::Project(ProjectMessage::ProjectEditPrSubmittedStallTimeoutSecondsChanged(
            pr_submitted_stall_timeout_seconds.saturating_sub(5).max(5),
        )),
        Message::Project(ProjectMessage::ProjectEditPrSubmittedStallTimeoutSecondsChanged(
            pr_submitted_stall_timeout_seconds.saturating_add(5).min(3600),
        )),
    );

    column![
        settings_group_heading(
            i18n.__("调度与保护"),
            i18n.__("限制并发、设置重试与超时，并控制 worktree 回收与合并锁。")
        ),
        settings_panel(
            column![
                workflow_controls,
                scheduler_tick_interval_row,
                auto_promote_tick_interval_row,
                max_concurrent_row,
                failed_retry_row,
                failed_retry_max_attempts_row,
                running_timeout_row,
                recycle_worktree_on_finish_row,
                pr_stall_timeout_row,
            ]
            .spacing(14)
        )
    ]
    .spacing(12)
    .into()
}

fn workflow_cache_hint(app: &App) -> Option<String> {
    if app.workflow_manager.lifecycles_loading() {
        Some(
            crate::app::i18n::I18n::new(app.desktop_locale)
                .__("生命周期列表正在从网关加载。")
                .to_string(),
        )
    } else if let Some(error) = app.workflow_manager.lifecycles_error() {
        Some(
            crate::app::i18n::I18n::new(app.desktop_locale)
                .format("生命周期列表加载失败：%s", &[error.into()])
                .expect("valid lifecycle error template"),
        )
    } else if !app.workflow_manager.lifecycles_loaded() {
        Some(
            crate::app::i18n::I18n::new(app.desktop_locale)
                .__("生命周期列表尚未从网关加载，当前显示本地缓存。")
                .to_string(),
        )
    } else {
        None
    }
}
#[cfg(test)]
#[path = "scheduling_tab_tests.rs"]
mod scheduling_tab_tests;
