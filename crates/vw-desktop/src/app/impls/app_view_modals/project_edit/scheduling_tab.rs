//! 渲染应用视图中的模态窗口。
//! 本模块只描述模态内容和交互消息，不直接承担持久化策略。

use crate::app::components::system_settings_common::{
    rounded_action_btn_style, settings_checkbox_style, settings_muted_text_style, settings_panel,
    settings_pick_list_menu_style, settings_pick_list_style, settings_section_card,
    settings_text_input_style,
};
use iced::widget::{Space, button, checkbox, column, pick_list, row, text, text_input};
use iced::{Element, Length};

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
    let workflow_row = row![
        text("任务工作流").size(13).width(Length::Fill),
        pick_list(workflow_options, selected_workflow, |option| {
            Message::Project(
                crate::app::message::project::ProjectMessage::ProjectEditWorkflowIdChanged(
                    option.id,
                ),
            )
        })
        .padding([8, 10])
        .text_size(13)
        .style(settings_pick_list_style)
        .menu_style(settings_pick_list_menu_style)
        .width(Length::Fixed(200.0)),
        button(text("管理工作流").size(12))
            .on_press(Message::View(crate::app::message::view::ViewMessage::OpenWorkflowManager))
            .padding([6, 10])
            .style(rounded_action_btn_style),
    ]
    .spacing(6)
    .align_y(iced::Alignment::Center);
    let workflow_cache_hint: Element<'_, Message> = workflow_cache_hint(app)
        .map(|hint| text(hint).size(11).style(settings_muted_text_style).into())
        .unwrap_or_else(|| Space::new().height(Length::Shrink).into());

    let max_concurrent_row = row![
        text("并发任务数").size(13).width(Length::Fill),
        button(text("-").size(12))
            .on_press(Message::Project(
                crate::app::message::project::ProjectMessage::ProjectEditMaxConcurrentChanged(
                    if max_concurrent <= 1 { 1 } else { max_concurrent - 1 },
                ),
            ))
            .padding([3, 8])
            .style(rounded_action_btn_style),
        text_input("1-10", &app.project_edit_max_concurrent_input)
            .on_input(|value| {
                Message::Project(
                    crate::app::message::project::ProjectMessage::ProjectEditMaxConcurrentInputChanged(
                        value,
                    ),
                )
            })
            .padding([8, 10])
            .size(13)
            .width(Length::Fixed(72.0))
            .style(settings_text_input_style),
        button(text("+").size(12))
            .on_press(Message::Project(
                crate::app::message::project::ProjectMessage::ProjectEditMaxConcurrentChanged(
                    if max_concurrent >= 10 { 10 } else { max_concurrent + 1 },
                ),
            ))
            .padding([3, 8])
            .style(rounded_action_btn_style),
    ]
    .spacing(6)
    .align_y(iced::Alignment::Center);

    let scheduler_tick_interval_row = row![
        text("执行唤醒间隔(秒)").size(13).width(Length::Fill),
        button(text("-").size(12))
            .on_press(Message::Project(
                crate::app::message::project::ProjectMessage::ProjectEditTaskBoardSchedulerTickIntervalSecondsChanged(
                    scheduler_tick_interval_seconds.saturating_sub(1).max(1),
                ),
            ))
            .padding([3, 8])
            .style(rounded_action_btn_style),
        text_input(
            "1-60",
            &app.project_edit_task_board_scheduler_tick_interval_seconds_input,
        )
        .on_input(|value| {
            Message::Project(
                crate::app::message::project::ProjectMessage::ProjectEditTaskBoardSchedulerTickIntervalSecondsInputChanged(
                    value,
                ),
            )
        })
        .padding([8, 10])
        .size(13)
        .width(Length::Fixed(72.0))
        .style(settings_text_input_style),
        button(text("+").size(12))
            .on_press(Message::Project(
                crate::app::message::project::ProjectMessage::ProjectEditTaskBoardSchedulerTickIntervalSecondsChanged(
                    scheduler_tick_interval_seconds.saturating_add(1).min(60),
                ),
            ))
            .padding([3, 8])
            .style(rounded_action_btn_style),
    ]
    .spacing(6)
    .align_y(iced::Alignment::Center);

    let auto_promote_tick_interval_row = row![
        text("任务池回推间隔(秒)").size(13).width(Length::Fill),
        button(text("-").size(12))
            .on_press(Message::Project(
                crate::app::message::project::ProjectMessage::ProjectEditTaskBoardAutoPromoteTickIntervalSecondsChanged(
                    auto_promote_tick_interval_seconds.saturating_sub(5).max(1),
                ),
            ))
            .padding([3, 8])
            .style(rounded_action_btn_style),
        text_input(
            "1-3600",
            &app.project_edit_task_board_auto_promote_tick_interval_seconds_input,
        )
        .on_input(|value| {
            Message::Project(
                crate::app::message::project::ProjectMessage::ProjectEditTaskBoardAutoPromoteTickIntervalSecondsInputChanged(
                    value,
                ),
            )
        })
        .padding([8, 10])
        .size(13)
        .width(Length::Fixed(72.0))
        .style(settings_text_input_style),
        button(text("+").size(12))
            .on_press(Message::Project(
                crate::app::message::project::ProjectMessage::ProjectEditTaskBoardAutoPromoteTickIntervalSecondsChanged(
                    auto_promote_tick_interval_seconds.saturating_add(5).min(3600),
                ),
            ))
            .padding([3, 8])
            .style(rounded_action_btn_style),
    ]
    .spacing(6)
    .align_y(iced::Alignment::Center);

    let failed_retry_row = row![
        text("失败重试(分钟)").size(13).width(Length::Fill),
        button(text("-").size(12))
            .on_press(Message::Project(
                crate::app::message::project::ProjectMessage::ProjectEditFailedRetryMinutesChanged(
                    if failed_retry_minutes <= 1 { 1 } else { failed_retry_minutes - 1 },
                ),
            ))
            .padding([3, 8])
            .style(rounded_action_btn_style),
        text_input("1-1440", &app.project_edit_failed_retry_minutes_input)
            .on_input(|value| {
                Message::Project(
                    crate::app::message::project::ProjectMessage::ProjectEditFailedRetryMinutesInputChanged(
                        value,
                    ),
                )
            })
            .padding([8, 10])
            .size(13)
            .width(Length::Fixed(72.0))
            .style(settings_text_input_style),
        button(text("+").size(12))
            .on_press(Message::Project(
                crate::app::message::project::ProjectMessage::ProjectEditFailedRetryMinutesChanged(
                    if failed_retry_minutes >= 1440 { 1440 } else { failed_retry_minutes + 1 },
                ),
            ))
            .padding([3, 8])
            .style(rounded_action_btn_style),
    ]
    .spacing(6)
    .align_y(iced::Alignment::Center);

    let failed_retry_max_attempts_row = row![
        text("最大重试(次)").size(13).width(Length::Fill),
        button(text("-").size(12))
            .on_press(Message::Project(
                crate::app::message::project::ProjectMessage::ProjectEditFailedRetryMaxAttemptsChanged(
                    failed_retry_max_attempts.saturating_sub(1),
                ),
            ))
            .padding([3, 8])
            .style(rounded_action_btn_style),
        text_input("0-10", &app.project_edit_failed_retry_max_attempts_input)
            .on_input(|value| {
                Message::Project(
                    crate::app::message::project::ProjectMessage::ProjectEditFailedRetryMaxAttemptsInputChanged(
                        value,
                    ),
                )
            })
            .padding([8, 10])
            .size(13)
            .width(Length::Fixed(72.0))
            .style(settings_text_input_style),
        button(text("+").size(12))
            .on_press(Message::Project(
                crate::app::message::project::ProjectMessage::ProjectEditFailedRetryMaxAttemptsChanged(
                    failed_retry_max_attempts.saturating_add(1).min(10),
                ),
            ))
            .padding([3, 8])
            .style(rounded_action_btn_style),
    ]
    .spacing(6)
    .align_y(iced::Alignment::Center);

    let running_timeout_row = row![
        text("运行超时(分钟)").size(13).width(Length::Fill),
        button(text("-").size(12))
            .on_press(Message::Project(
                crate::app::message::project::ProjectMessage::ProjectEditRunningTimeoutMinutesChanged(
                    if running_timeout_minutes <= 1 { 1 } else { running_timeout_minutes - 1 },
                ),
            ))
            .padding([3, 8])
            .style(rounded_action_btn_style),
        text_input("1-1440", &app.project_edit_running_timeout_minutes_input)
            .on_input(|value| {
                Message::Project(
                    crate::app::message::project::ProjectMessage::ProjectEditRunningTimeoutMinutesInputChanged(
                        value,
                    ),
                )
            })
            .padding([8, 10])
            .size(13)
            .width(Length::Fixed(72.0))
            .style(settings_text_input_style),
        button(text("+").size(12))
            .on_press(Message::Project(
                crate::app::message::project::ProjectMessage::ProjectEditRunningTimeoutMinutesChanged(
                    if running_timeout_minutes >= 1440 {
                        1440
                    } else {
                        running_timeout_minutes + 1
                    },
                ),
            ))
            .padding([3, 8])
            .style(rounded_action_btn_style),
    ]
    .spacing(6)
    .align_y(iced::Alignment::Center);

    let recycle_worktree_on_finish_toggle = checkbox(settings.recycle_worktree_on_task_finish)
        .label("完成后回收 worktree")
        .spacing(8)
        .on_toggle(|enabled| {
            Message::Project(
                crate::app::message::project::ProjectMessage::ProjectEditRecycleWorktreeOnTaskFinishToggled(
                    enabled,
                ),
            )
        })
        .style(settings_checkbox_style);

    let pr_stall_timeout_row = row![
        text("合并锁超时(秒)").size(13).width(Length::Fill),
        button(text("-").size(12))
            .on_press(Message::Project(
                crate::app::message::project::ProjectMessage::ProjectEditPrSubmittedStallTimeoutSecondsChanged(
                    pr_submitted_stall_timeout_seconds.saturating_sub(5).max(5),
                ),
            ))
            .padding([3, 8])
            .style(rounded_action_btn_style),
        text_input("5-3600", &app.project_edit_pr_submitted_stall_timeout_seconds_input)
            .on_input(|value| {
                Message::Project(
                    crate::app::message::project::ProjectMessage::ProjectEditPrSubmittedStallTimeoutSecondsInputChanged(
                        value,
                    ),
                )
            })
            .padding([8, 10])
            .size(13)
            .width(Length::Fixed(72.0))
            .style(settings_text_input_style),
        button(text("+").size(12))
            .on_press(Message::Project(
                crate::app::message::project::ProjectMessage::ProjectEditPrSubmittedStallTimeoutSecondsChanged(
                    pr_submitted_stall_timeout_seconds.saturating_add(5).min(3600),
                ),
            ))
            .padding([3, 8])
            .style(rounded_action_btn_style),
    ]
    .spacing(6)
    .align_y(iced::Alignment::Center);

    column![
        settings_section_card(
            "调度与保护",
            "限制并发、设置重试与超时，并控制 worktree 回收与合并锁。"
        ),
        settings_panel(
            column![
                workflow_row,
                workflow_cache_hint,
                scheduler_tick_interval_row,
                auto_promote_tick_interval_row,
                max_concurrent_row,
                failed_retry_row,
                failed_retry_max_attempts_row,
                running_timeout_row,
                recycle_worktree_on_finish_toggle,
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
        Some("生命周期列表正在从网关加载。".to_string())
    } else if let Some(error) = app.workflow_manager.lifecycles_error() {
        Some(format!("生命周期列表加载失败：{error}"))
    } else if !app.workflow_manager.lifecycles_loaded() {
        Some("生命周期列表尚未从网关加载，当前显示本地缓存。".to_string())
    } else {
        None
    }
}
#[cfg(test)]
#[path = "scheduling_tab_tests.rs"]
mod scheduling_tab_tests;
