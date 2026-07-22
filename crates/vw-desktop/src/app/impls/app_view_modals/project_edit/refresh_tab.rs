//! 渲染应用视图中的模态窗口。
//! 本模块只描述模态内容和交互消息，不直接承担持久化策略。

use crate::app::components::system_settings_common::{
    rounded_action_btn_style, settings_panel, settings_section_card, settings_text_input_style,
};
use iced::widget::{button, column, row, text, text_input, toggler};
use iced::{Element, Length};

use super::{App, Message};

fn normalized_session_refresh_interval(input: &str) -> u64 {
    input.trim().parse::<u64>().ok().map(|value| value.clamp(1, 3600)).unwrap_or(60)
}

fn refresh_interval_step_targets(value: u64) -> (u64, u64) {
    let value = value.clamp(1, 3600);
    (value.saturating_sub(5).max(1), value.saturating_add(5).min(3600))
}

fn task_board_auto_refresh_toggled(value: bool) -> Message {
    Message::Project(
        crate::app::message::project::ProjectMessage::ProjectEditTaskBoardAutoRefreshToggled(value),
    )
}

fn session_auto_refresh_toggled(value: bool) -> Message {
    Message::Project(
        crate::app::message::project::ProjectMessage::ProjectEditSessionAutoRefreshToggled(value),
    )
}

fn session_refresh_interval_changed(value: u64) -> Message {
    Message::Project(
        crate::app::message::project::ProjectMessage::ProjectEditSessionRefreshIntervalSecondsChanged(
            value,
        ),
    )
}

fn session_refresh_interval_input_changed(value: String) -> Message {
    Message::Project(
        crate::app::message::project::ProjectMessage::ProjectEditSessionRefreshIntervalSecondsInputChanged(
            value,
        ),
    )
}

fn task_board_refresh_interval_changed(value: u64) -> Message {
    Message::Project(
        crate::app::message::project::ProjectMessage::ProjectEditTaskBoardRefreshIntervalSecondsChanged(
            value,
        ),
    )
}

fn task_board_refresh_interval_input_changed(value: String) -> Message {
    Message::Project(
        crate::app::message::project::ProjectMessage::ProjectEditTaskBoardRefreshIntervalSecondsInputChanged(
            value,
        ),
    )
}

/// 模块内可见函数，执行 refresh_tab 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn refresh_tab<'a>(app: &'a App) -> Element<'a, Message> {
    let settings = &app.project_edit_task_board_settings;
    let session_refresh_interval_seconds = normalized_session_refresh_interval(
        &app.project_edit_session_refresh_interval_seconds_input,
    );
    let task_board_refresh_interval_seconds = settings.refresh_interval_seconds.clamp(1, 3600);
    let (session_refresh_interval_decrease, session_refresh_interval_increase) =
        refresh_interval_step_targets(session_refresh_interval_seconds);
    let (task_board_refresh_interval_decrease, task_board_refresh_interval_increase) =
        refresh_interval_step_targets(task_board_refresh_interval_seconds);

    let task_board_auto_refresh_toggle = row![
        text("任务看板自动刷新").size(13).width(Length::Fill),
        toggler(app.project_edit_task_board_auto_refresh)
            .on_toggle(task_board_auto_refresh_toggled)
    ]
    .align_y(iced::Alignment::Center);

    let session_auto_refresh_toggle = row![
        text("项目会话自动刷新").size(13).width(Length::Fill),
        toggler(app.project_edit_session_auto_refresh).on_toggle(session_auto_refresh_toggled)
    ]
    .align_y(iced::Alignment::Center);

    let session_refresh_interval_row = row![
        text("会话刷新(秒)").size(13).width(Length::Fill),
        button(text("-").size(12))
            .on_press(session_refresh_interval_changed(session_refresh_interval_decrease,))
            .padding([3, 8])
            .style(rounded_action_btn_style),
        text_input("1-3600", &app.project_edit_session_refresh_interval_seconds_input)
            .on_input(session_refresh_interval_input_changed)
            .padding([8, 10])
            .size(13)
            .width(Length::Fixed(72.0))
            .style(settings_text_input_style),
        button(text("+").size(12))
            .on_press(session_refresh_interval_changed(session_refresh_interval_increase,))
            .padding([3, 8])
            .style(rounded_action_btn_style),
    ]
    .spacing(6)
    .align_y(iced::Alignment::Center);

    let task_board_refresh_interval_row = row![
        text("任务看板刷新(秒)").size(13).width(Length::Fill),
        button(text("-").size(12))
            .on_press(task_board_refresh_interval_changed(task_board_refresh_interval_decrease,))
            .padding([3, 8])
            .style(rounded_action_btn_style),
        text_input("1-3600", &app.project_edit_task_board_refresh_interval_seconds_input)
            .on_input(task_board_refresh_interval_input_changed)
            .padding([8, 10])
            .size(13)
            .width(Length::Fixed(72.0))
            .style(settings_text_input_style),
        button(text("+").size(12))
            .on_press(task_board_refresh_interval_changed(task_board_refresh_interval_increase,))
            .padding([3, 8])
            .style(rounded_action_btn_style),
    ]
    .spacing(6)
    .align_y(iced::Alignment::Center);

    column![
        settings_section_card("刷新策略", "统一项目会话与任务看板刷新策略。"),
        settings_panel(
            column![
                task_board_auto_refresh_toggle,
                session_auto_refresh_toggle,
                session_refresh_interval_row,
                task_board_refresh_interval_row,
            ]
            .spacing(14)
        )
    ]
    .spacing(12)
    .into()
}
#[cfg(test)]
#[path = "refresh_tab_tests.rs"]
mod refresh_tab_tests;
