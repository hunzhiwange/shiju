//! 任务看板的设置弹窗与任务上下文菜单。

use iced::widget::{Space, button, checkbox, column, container, row, text, text_input};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};

use crate::app::components::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, rounded_action_btn_style,
    settings_checkbox_style, settings_close_button, settings_divider, settings_field_label,
    settings_group_card, settings_group_heading, settings_modal_header, settings_modal_layout,
    settings_muted_text_style, settings_segment_button_style, settings_tab_button,
    settings_tab_row, settings_text_input_style, settings_toggle,
};
use crate::app::i18n::I18n;
use crate::app::message::TaskBoardMessage;
use crate::app::state::TaskBoardSettingsModalTab;
use crate::app::task::Task;
use crate::app::{App, Message};
use vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerDispatchModeDto;

use super::common::button_style_secondary;

/// 构建任务设置模态窗口
///
/// 该函数创建一个任务设置模态窗口，用于配置任务看板的全局执行参数。
/// 这些设置会影响所有任务的执行行为
///
/// # 可配置参数
///
/// - **并发数**：同时执行的最大任务数量（1-10）
/// - **失败回推(分)**：失败任务重新进入待执行队列的等待时间（1-1440 分钟）
/// - **执行超时(分)**：单个任务的最大执行时间（1-1440 分钟）
/// - **自动回推延迟(秒)**：任务状态自动转换的延迟时间（0-3600 秒）
///
/// # 参数
///
/// * `app` - 应用程序状态的不可变引用，用于获取当前设置值
///
/// # 返回值
///
/// 返回构建好的设置模态窗口 UI 元素，类型为 `Element<'_, Message>`
///
/// # 示例
///
/// ```ignore
/// let settings_modal = build_task_settings_modal(&app);
/// // 将 settings_modal 添加到 UI 树中进行渲染
/// ```
pub fn build_task_settings_modal(app: &App) -> Element<'_, Message> {
    fn field_row<'a>(
        label: &'a str,
        control: impl Into<Element<'a, Message>>,
    ) -> Element<'a, Message> {
        row![container(settings_field_label(label)).width(Length::Fill), control.into(),]
            .spacing(12)
            .align_y(Alignment::Center)
            .padding([10, 0])
            .into()
    }

    let i18n = I18n::new(app.desktop_locale);
    let tab_button = |label: &'static str, tab: TaskBoardSettingsModalTab| {
        settings_tab_button(
            i18n.__(label),
            app.task_board_settings_modal_tab == tab,
            Some(Message::TaskBoard(TaskBoardMessage::SelectSettingsModalTab(tab))),
        )
        .into()
    };

    // 构建并发数设置行
    // 包含减少按钮、输入框和增加按钮，值范围为 1-10
    let max_concurrent = app.task_board_settings.max_concurrent;
    let max_concurrent_text = max_concurrent.to_string();
    let current_refresh_interval_seconds =
        app.task_board_settings.refresh_interval_seconds.clamp(1, 3600);
    let refresh_interval_seconds_text = current_refresh_interval_seconds.to_string();
    let current_scheduler_tick_interval_seconds =
        app.task_board_settings.scheduler_tick_interval_seconds.clamp(1, 60);
    let scheduler_tick_interval_seconds_text = current_scheduler_tick_interval_seconds.to_string();
    let current_auto_promote_tick_interval_seconds =
        app.task_board_settings.auto_promote_tick_interval_seconds.clamp(1, 3600);
    let auto_promote_tick_interval_seconds_text =
        current_auto_promote_tick_interval_seconds.to_string();
    let max_concurrent_row = field_row(
        i18n.__("并发数"),
        row![
            button(text("-").size(12))
                .on_press(Message::TaskBoard(TaskBoardMessage::SetMaxConcurrent(
                    max_concurrent.saturating_sub(1).max(1),
                )))
                .padding([8, 10])
                .style(rounded_action_btn_style),
            text_input(i18n.__("数量"), &max_concurrent_text)
                .on_input(move |value| {
                    // 解析输入值并限制在有效范围内
                    let count = value
                        .trim()
                        .parse::<u32>()
                        .ok()
                        .map(|v| v.clamp(1, 10))
                        .unwrap_or(max_concurrent.clamp(1, 10));
                    Message::TaskBoard(TaskBoardMessage::SetMaxConcurrent(count))
                })
                .padding(SETTINGS_CONTROL_PADDING)
                .size(SETTINGS_CONTROL_TEXT_SIZE)
                .width(Length::Fixed(76.0))
                .style(settings_text_input_style),
            button(text("+").size(12))
                .on_press(Message::TaskBoard(TaskBoardMessage::SetMaxConcurrent(
                    max_concurrent.saturating_add(1).min(10),
                )))
                .padding([8, 10])
                .style(rounded_action_btn_style),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    );

    let auto_refresh_toggle_row = field_row(
        i18n.__("自动刷新"),
        row![settings_toggle(app.task_board_settings.auto_refresh).on_toggle(|enabled| {
            Message::TaskBoard(TaskBoardMessage::ToggleAutoRefresh(enabled))
        }),]
        .spacing(8)
        .align_y(Alignment::Center),
    );

    let dispatch_mode = app.task_board_settings.agent_scheduler_dispatch_mode;
    let dispatch_mode_button = |label: &'static str, mode: AgentSchedulerDispatchModeDto| {
        let active = dispatch_mode == mode;
        button(text(label).size(12))
            .on_press(Message::TaskBoard(TaskBoardMessage::SetAgentSchedulerDispatchMode(mode)))
            .padding([8, 12])
            .style(move |theme, status| settings_segment_button_style(theme, status, active))
    };
    let dispatch_mode_row = field_row(
        i18n.__("调度方式"),
        row![
            dispatch_mode_button(i18n.__("本地"), AgentSchedulerDispatchModeDto::Local),
            dispatch_mode_button(i18n.__("自动"), AgentSchedulerDispatchModeDto::Auto),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    );

    let refresh_interval_row = field_row(
        i18n.__("刷新间隔(秒)"),
        row![
            button(text("-").size(12))
                .on_press(Message::TaskBoard(TaskBoardMessage::SetRefreshIntervalSeconds(
                    current_refresh_interval_seconds.saturating_sub(5).max(1),
                )))
                .padding([8, 10])
                .style(rounded_action_btn_style),
            text_input(i18n.__("秒"), &refresh_interval_seconds_text)
                .on_input(move |value| {
                    let seconds = value
                        .trim()
                        .parse::<u64>()
                        .ok()
                        .map(|v| v.clamp(1, 3600))
                        .unwrap_or(current_refresh_interval_seconds);
                    Message::TaskBoard(TaskBoardMessage::SetRefreshIntervalSeconds(seconds))
                })
                .padding(SETTINGS_CONTROL_PADDING)
                .size(SETTINGS_CONTROL_TEXT_SIZE)
                .width(Length::Fixed(76.0))
                .style(settings_text_input_style),
            button(text("+").size(12))
                .on_press(Message::TaskBoard(TaskBoardMessage::SetRefreshIntervalSeconds(
                    current_refresh_interval_seconds.saturating_add(5).min(3600),
                )))
                .padding([8, 10])
                .style(rounded_action_btn_style),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    );

    let scheduler_tick_interval_row = field_row(
        i18n.__("执行唤醒间隔(秒)"),
        row![
            button(text("-").size(12))
                .on_press(Message::TaskBoard(TaskBoardMessage::SetSchedulerTickIntervalSeconds(
                    current_scheduler_tick_interval_seconds.saturating_sub(1).max(1),
                )))
                .padding([8, 10])
                .style(rounded_action_btn_style),
            text_input(i18n.__("秒"), &scheduler_tick_interval_seconds_text)
                .on_input(move |value| {
                    let seconds = value
                        .trim()
                        .parse::<u64>()
                        .ok()
                        .map(|v| v.clamp(1, 60))
                        .unwrap_or(current_scheduler_tick_interval_seconds);
                    Message::TaskBoard(TaskBoardMessage::SetSchedulerTickIntervalSeconds(seconds))
                })
                .padding(SETTINGS_CONTROL_PADDING)
                .size(SETTINGS_CONTROL_TEXT_SIZE)
                .width(Length::Fixed(76.0))
                .style(settings_text_input_style),
            button(text("+").size(12))
                .on_press(Message::TaskBoard(TaskBoardMessage::SetSchedulerTickIntervalSeconds(
                    current_scheduler_tick_interval_seconds.saturating_add(1).min(60),
                )))
                .padding([8, 10])
                .style(rounded_action_btn_style),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    );

    let auto_promote_tick_interval_row = field_row(
        i18n.__("任务池回推间隔(秒)"),
        row![
            button(text("-").size(12))
                .on_press(Message::TaskBoard(TaskBoardMessage::SetAutoPromoteTickIntervalSeconds(
                    current_auto_promote_tick_interval_seconds.saturating_sub(5).max(1),
                )))
                .padding([8, 10])
                .style(rounded_action_btn_style),
            text_input(i18n.__("秒"), &auto_promote_tick_interval_seconds_text)
                .on_input(move |value| {
                    let seconds = value
                        .trim()
                        .parse::<u64>()
                        .ok()
                        .map(|v| v.clamp(1, 3600))
                        .unwrap_or(current_auto_promote_tick_interval_seconds);
                    Message::TaskBoard(TaskBoardMessage::SetAutoPromoteTickIntervalSeconds(seconds))
                })
                .padding(SETTINGS_CONTROL_PADDING)
                .size(SETTINGS_CONTROL_TEXT_SIZE)
                .width(Length::Fixed(76.0))
                .style(settings_text_input_style),
            button(text("+").size(12))
                .on_press(Message::TaskBoard(TaskBoardMessage::SetAutoPromoteTickIntervalSeconds(
                    current_auto_promote_tick_interval_seconds.saturating_add(5).min(3600),
                )))
                .padding([8, 10])
                .style(rounded_action_btn_style),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    );

    // 构建失败回推时间设置行
    // 值范围为 1-1440 分钟（最多 24 小时）
    let current_retry_minutes = app.task_board_settings.failed_retry_minutes;
    let retry_minutes_text = current_retry_minutes.to_string();
    let retry_row = field_row(
        i18n.__("失败回推(分)"),
        row![
            button(text("-").size(12))
                .on_press(Message::TaskBoard(TaskBoardMessage::SetFailedRetryMinutes(
                    current_retry_minutes.saturating_sub(1).max(1),
                )))
                .padding([8, 10])
                .style(rounded_action_btn_style),
            text_input(i18n.__("分钟"), &retry_minutes_text)
                .on_input(move |value| {
                    // 解析输入值并限制在有效范围内
                    let minutes = value
                        .trim()
                        .parse::<u32>()
                        .ok()
                        .map(|v| v.clamp(1, 1440))
                        .unwrap_or(current_retry_minutes.clamp(1, 1440));
                    Message::TaskBoard(TaskBoardMessage::SetFailedRetryMinutes(minutes))
                })
                .padding(SETTINGS_CONTROL_PADDING)
                .size(SETTINGS_CONTROL_TEXT_SIZE)
                .width(Length::Fixed(76.0))
                .style(settings_text_input_style),
            button(text("+").size(12))
                .on_press(Message::TaskBoard(TaskBoardMessage::SetFailedRetryMinutes(
                    current_retry_minutes.saturating_add(1).min(1440),
                )))
                .padding([8, 10])
                .style(rounded_action_btn_style),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    );

    let current_retry_max_attempts = app.task_board_settings.failed_retry_max_attempts;
    let retry_max_attempts_text = current_retry_max_attempts.to_string();
    let retry_max_attempts_row = field_row(
        i18n.__("最大重试(次)"),
        row![
            button(text("-").size(12))
                .on_press(Message::TaskBoard(TaskBoardMessage::SetFailedRetryMaxAttempts(
                    current_retry_max_attempts.saturating_sub(1),
                )))
                .padding([8, 10])
                .style(rounded_action_btn_style),
            text_input("0-10", &retry_max_attempts_text)
                .on_input(move |value| {
                    let attempts = value
                        .trim()
                        .parse::<u32>()
                        .ok()
                        .map(|v| v.clamp(0, 10))
                        .unwrap_or(current_retry_max_attempts.clamp(0, 10));
                    Message::TaskBoard(TaskBoardMessage::SetFailedRetryMaxAttempts(attempts))
                })
                .padding(SETTINGS_CONTROL_PADDING)
                .size(SETTINGS_CONTROL_TEXT_SIZE)
                .width(Length::Fixed(76.0))
                .style(settings_text_input_style),
            button(text("+").size(12))
                .on_press(Message::TaskBoard(TaskBoardMessage::SetFailedRetryMaxAttempts(
                    current_retry_max_attempts.saturating_add(1).min(10),
                )))
                .padding([8, 10])
                .style(rounded_action_btn_style),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    );

    // 构建执行超时时间设置行
    // 值范围为 1-1440 分钟（最多 24 小时）
    let current_timeout_minutes = app.task_board_settings.running_timeout_minutes;
    let timeout_minutes_text = current_timeout_minutes.to_string();
    let timeout_row = field_row(
        i18n.__("执行超时(分)"),
        row![
            button(text("-").size(12))
                .on_press(Message::TaskBoard(TaskBoardMessage::SetRunningTimeoutMinutes(
                    current_timeout_minutes.saturating_sub(1).max(1),
                )))
                .padding([8, 10])
                .style(rounded_action_btn_style),
            text_input(i18n.__("分钟"), &timeout_minutes_text)
                .on_input(move |value| {
                    // 解析输入值并限制在有效范围内
                    let minutes = value
                        .trim()
                        .parse::<u32>()
                        .ok()
                        .map(|v| v.clamp(1, 1440))
                        .unwrap_or(current_timeout_minutes.clamp(1, 1440));
                    Message::TaskBoard(TaskBoardMessage::SetRunningTimeoutMinutes(minutes))
                })
                .padding(SETTINGS_CONTROL_PADDING)
                .size(SETTINGS_CONTROL_TEXT_SIZE)
                .width(Length::Fixed(76.0))
                .style(settings_text_input_style),
            button(text("+").size(12))
                .on_press(Message::TaskBoard(TaskBoardMessage::SetRunningTimeoutMinutes(
                    current_timeout_minutes.saturating_add(1).min(1440),
                )))
                .padding([8, 10])
                .style(rounded_action_btn_style),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    );

    let recycle_on_finish_row = field_row(
        i18n.__("完成后回收 worktree"),
        checkbox(app.task_board_settings.recycle_worktree_on_task_finish)
            .on_toggle(|enabled| {
                Message::TaskBoard(TaskBoardMessage::ToggleRecycleWorktreeOnTaskFinish(enabled))
            })
            .style(settings_checkbox_style),
    );

    let current_pr_stall_timeout_seconds =
        app.task_board_settings.pr_submitted_stall_timeout_seconds;
    let pr_stall_timeout_seconds_text = current_pr_stall_timeout_seconds.to_string();
    let pr_stall_timeout_row = field_row(
        i18n.__("合并锁超时(秒)"),
        row![
            button(text("-").size(12))
                .on_press(Message::TaskBoard(TaskBoardMessage::SetPrSubmittedStallTimeoutSeconds(
                    current_pr_stall_timeout_seconds.saturating_sub(5).max(5),
                )))
                .padding([8, 10])
                .style(rounded_action_btn_style),
            text_input(i18n.__("秒"), &pr_stall_timeout_seconds_text)
                .on_input(move |value| {
                    let seconds = value
                        .trim()
                        .parse::<u32>()
                        .ok()
                        .map(|v| v.clamp(5, 3600))
                        .unwrap_or(current_pr_stall_timeout_seconds.clamp(5, 3600));
                    Message::TaskBoard(TaskBoardMessage::SetPrSubmittedStallTimeoutSeconds(seconds))
                })
                .padding(SETTINGS_CONTROL_PADDING)
                .size(SETTINGS_CONTROL_TEXT_SIZE)
                .width(Length::Fixed(76.0))
                .style(settings_text_input_style),
            button(text("+").size(12))
                .on_press(Message::TaskBoard(TaskBoardMessage::SetPrSubmittedStallTimeoutSeconds(
                    current_pr_stall_timeout_seconds.saturating_add(5).min(3600),
                )))
                .padding([8, 10])
                .style(rounded_action_btn_style),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    );

    // 构建自动回推延迟设置行
    // 值范围为 0-3600 秒（最多 1 小时）
    let current_delay_seconds = app.task_board_settings.auto_promote_delay_seconds;
    let delay_seconds_text = current_delay_seconds.to_string();
    let delay_row = field_row(
        i18n.__("自动回推延迟(秒)"),
        row![
            button(text("-").size(12))
                .on_press(Message::TaskBoard(TaskBoardMessage::SetAutoPromoteDelay(
                    current_delay_seconds.saturating_sub(5),
                )))
                .padding([8, 10])
                .style(rounded_action_btn_style),
            text_input(i18n.__("秒"), &delay_seconds_text)
                .on_input(move |value| {
                    // 解析输入值并限制在有效范围内
                    let seconds = value
                        .trim()
                        .parse::<u64>()
                        .ok()
                        .map(|v| v.clamp(0, 3600))
                        .unwrap_or(current_delay_seconds.clamp(0, 3600));
                    Message::TaskBoard(TaskBoardMessage::SetAutoPromoteDelay(seconds))
                })
                .padding(SETTINGS_CONTROL_PADDING)
                .size(SETTINGS_CONTROL_TEXT_SIZE)
                .width(Length::Fixed(76.0))
                .style(settings_text_input_style),
            button(text("+").size(12))
                .on_press(Message::TaskBoard(TaskBoardMessage::SetAutoPromoteDelay(
                    current_delay_seconds.saturating_add(5).min(3600),
                )))
                .padding([8, 10])
                .style(rounded_action_btn_style),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    );

    let (heading, description, rows) = match app.task_board_settings_modal_tab {
        TaskBoardSettingsModalTab::Refresh => (
            "刷新与自动化",
            "统一管理自动刷新、Agent Scheduler 唤醒与任务池回推节奏。",
            column![
                auto_refresh_toggle_row,
                settings_divider(),
                dispatch_mode_row,
                settings_divider(),
                refresh_interval_row,
                settings_divider(),
                scheduler_tick_interval_row,
                settings_divider(),
                auto_promote_tick_interval_row,
                settings_divider(),
                delay_row,
            ],
        ),
        TaskBoardSettingsModalTab::Scheduling => (
            "并发与保护",
            "控制任务吞吐、失败重试、超时与 worktree 回收策略。",
            column![
                max_concurrent_row,
                settings_divider(),
                retry_row,
                settings_divider(),
                retry_max_attempts_row,
                settings_divider(),
                timeout_row,
                settings_divider(),
                recycle_on_finish_row,
                settings_divider(),
                pr_stall_timeout_row,
            ],
        ),
    };

    let header = column![
        settings_modal_header(
            column![
                text(i18n.__("任务设置")).size(20),
                text(i18n.__("数值会立即作用于当前项目任务看板。"))
                    .size(13)
                    .style(settings_muted_text_style),
            ]
            .spacing(4),
            settings_close_button(Message::TaskBoard(TaskBoardMessage::CloseSettingsModal)),
        ),
        settings_tab_row([
            tab_button("刷新与自动化", TaskBoardSettingsModalTab::Refresh),
            tab_button("并发与保护", TaskBoardSettingsModalTab::Scheduling),
        ])
        .wrap()
        .vertical_spacing(8),
    ]
    .spacing(12);
    let body = column![
        settings_group_heading(i18n.__(heading), i18n.__(description)),
        settings_group_card(rows),
    ]
    .spacing(12);
    settings_modal_layout(header, body, None).width(580).height(520).into()
}

/// 构建任务上下文菜单
///
/// 该函数创建一个任务项的右键上下文菜单，提供常用的任务操作选项。
/// 菜单以弹出层形式显示，包含复制、删除和归档等操作
///
/// # 菜单选项
///
/// - **复制任务**：创建当前任务的副本
/// - **删除任务**：永久删除该任务
/// - **归档任务**：将任务移动到归档区
///
/// # 参数
///
/// * `task` - 要操作的任务对象，包含任务 ID 等必要信息
///
/// # 返回值
///
/// 返回构建好的上下文菜单 UI 元素，类型为 `Element<'static, Message>`
/// 使用 `'static` 生命周期是因为菜单内容不依赖于外部引用
///
/// # 示例
///
/// ```ignore
/// let context_menu = build_context_menu(task);
/// // 在右键点击事件中显示 context_menu
/// ```
pub fn build_context_menu(i18n: crate::app::i18n::I18n, task: Task) -> Element<'static, Message> {
    let retry_btn = (task.status == crate::app::task::TaskStatus::Failed).then(|| {
        button(
            row![text("↻").size(12), Space::new().width(8.0), text(i18n.__("手工重试")).size(12),]
                .align_y(Alignment::Center),
        )
        .on_press(Message::TaskBoard(TaskBoardMessage::ExecuteTask { task_id: task.id.clone() }))
        .padding([6, 10])
        .width(Length::Fill)
        .style(button_style_secondary)
    });

    // 复制任务按钮：点击触发复制任务的消息
    let copy_btn = button(
        row![text("📄").size(12), Space::new().width(8.0), text(i18n.__("复制任务")).size(12),]
            .align_y(Alignment::Center),
    )
    .on_press(Message::TaskBoard(TaskBoardMessage::DuplicateTask(task.id.clone())))
    .padding([6, 10])
    .width(Length::Fill)
    .style(button_style_secondary);

    // 删除任务按钮：点击触发删除任务的消息
    let delete_btn = button(
        row![text("🗑️").size(12), Space::new().width(8.0), text(i18n.__("删除任务")).size(12),]
            .align_y(Alignment::Center),
    )
    .on_press(Message::TaskBoard(TaskBoardMessage::TaskDeleted(task.id.clone())))
    .padding([6, 10])
    .width(Length::Fill)
    .style(button_style_secondary);

    // 归档任务按钮：点击触发归档任务的消息
    let archive_btn = button(
        row![text("📦").size(12), Space::new().width(8.0), text(i18n.__("归档任务")).size(12),]
            .align_y(Alignment::Center),
    )
    .on_press(Message::TaskBoard(TaskBoardMessage::TaskArchived(task.id.clone())))
    .padding([6, 10])
    .width(Length::Fill)
    .style(button_style_secondary);

    // 组装菜单项
    let mut menu = column![].spacing(2).width(Length::Fixed(132.0));
    if let Some(retry_btn) = retry_btn {
        menu = menu.push(retry_btn);
    }
    menu = menu.push(copy_btn).push(delete_btn).push(archive_btn);

    // 返回带有容器样式和阴影效果的完整菜单
    container(menu)
        .padding(6)
        .style(|theme: &Theme| container::Style {
            background: Some(Background::Color(theme.palette().background)),
            border: Border {
                width: 1.0,
                color: theme.extended_palette().background.strong.color,
                radius: 6.0.into(),
            },
            shadow: iced::Shadow {
                color: Color::BLACK.scale_alpha(0.18),
                offset: iced::Vector::new(1.0, 2.0),
                blur_radius: 6.0,
            },
            ..Default::default()
        })
        .into()
}

#[cfg(test)]
#[path = "modals_tests.rs"]
mod modals_tests;
