//! 任务看板控制区与员工视图。
mod helpers;
mod worktree_office;

use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Alignment, Color, Element, Length, Padding, Theme};

use desktop_ui_controls::system_settings_common::{
    settings_muted_text_style, settings_segment_button_style,
};
use crate::message::TaskBoardMessage;
use crate::message::TaskBoardMessage as Message;

use self::helpers::running_dots;
use self::worktree_office::build_worktree_pixel_office;
use super::common::{button_style_danger, button_style_primary, button_style_secondary};

mod styles;
use styles::*;

/// 构建任务看板控制区和员工视图。
pub fn build_control_buttons<'a>(app: &ControlView<'a>) -> Element<'a, Message> {
    let status_chip = |label: String, tone: Color| -> Element<'a, Message> {
        container(text(label).size(11))
            .padding([5, 10])
            .style(move |theme: &Theme| status_chip_style(theme, tone))
            .into()
    };

    let title = container(
        row![
            text(desktop_i18n::I18n::new(app.desktop_locale).__("任务看板"))
                .size(20)
                .font(iced::Font { weight: iced::font::Weight::Bold, ..Default::default() }),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    )
    .width(Length::Fill);

    let completed_count = app
        .task_board_tasks
        .iter()
        .filter(|task| {
            !task.deleted
                && !task.archived
                && task.status == desktop_task_model::TaskStatus::Completed
        })
        .count();
    let scheduler_chip = status_chip(
        if app.task_board_executor_running {
            desktop_i18n::I18n::new(app.desktop_locale).__("调度运行中").to_string()
        } else {
            desktop_i18n::I18n::new(app.desktop_locale).__("调度待启动").to_string()
        },
        if app.task_board_executor_running {
            Color::from_rgb8(0x2E, 0xB8, 0x72)
        } else {
            Color::from_rgba8(0, 0, 0, 0.0)
        },
    );
    let completed_chip = status_chip(
        desktop_i18n::I18n::new(app.desktop_locale)
            .format("待归档 %u", &[(completed_count).into()])
            .expect("valid task display template"),
        if completed_count > 0 {
            Color::from_rgb8(0xF2, 0xA9, 0x00)
        } else {
            Color::from_rgba8(0, 0, 0, 0.0)
        },
    );
    let i18n = desktop_i18n::I18n::new(app.desktop_locale);
    let office_selected = app.task_board_worktree_pixel_office;
    let board_btn = button(text(i18n.__("任务池")).size(12))
        .on_press_maybe(
            office_selected.then_some(TaskBoardMessage::ToggleWorktreePixelOffice(false)),
        )
        .padding([6, 10])
        .style(move |theme, status| settings_segment_button_style(theme, status, !office_selected));
    let office_btn = button(text(i18n.__("员工")).size(12))
        .on_press_maybe(
            (!office_selected).then_some(TaskBoardMessage::ToggleWorktreePixelOffice(true)),
        )
        .padding([6, 10])
        .style(move |theme, status| settings_segment_button_style(theme, status, office_selected));
    let view_switch = row![board_btn, office_btn].spacing(4).align_y(Alignment::Center);

    let settings_btn =
        button(text(desktop_i18n::I18n::new(app.desktop_locale).__("看板设置")).size(12))
            .on_press(TaskBoardMessage::OpenSettingsModal)
            .padding([6, 12])
            .style(button_style_secondary);

    let (exec_btn_label, exec_msg) = if app.task_board_executor_running {
        (
            desktop_i18n::I18n::new(app.desktop_locale).__("停止任务调度").to_string(),
            TaskBoardMessage::StopExecution,
        )
    } else {
        (
            desktop_i18n::I18n::new(app.desktop_locale).__("开始任务调度").to_string(),
            TaskBoardMessage::StartExecution,
        )
    };
    let exec_btn = button(text(exec_btn_label).size(12))
        .on_press(exec_msg)
        .padding([6, 12])
        .style(if app.task_board_executor_running {
            button_style_danger
        } else {
            button_style_primary
        });

    let new_task_btn =
        button(text(desktop_i18n::I18n::new(app.desktop_locale).__("+ 新建任务")).size(12))
            .on_press(TaskBoardMessage::CreateTask)
            .padding([6, 12])
            .style(button_style_primary);

    let archive_completed_btn = button(
        text(
            desktop_i18n::I18n::new(app.desktop_locale)
                .format("归档已完成 %u", &[(completed_count).into()])
                .expect("valid task display template"),
        )
        .size(12),
    )
    .on_press_maybe(
        (completed_count > 0)
            .then_some(TaskBoardMessage::ArchiveCompletedTasks),
    )
    .padding([6, 12])
    .style(button_style_secondary);

    let refresh_btn =
        button(text(desktop_i18n::I18n::new(app.desktop_locale).__("刷新任务")).size(12))
            .on_press(TaskBoardMessage::LoadTasks)
            .padding([6, 12])
            .style(button_style_secondary);

    let close_btn =
        button(text(desktop_i18n::I18n::new(app.desktop_locale).__("关闭看板")).size(12))
            .on_press(TaskBoardMessage::CloseBoard)
            .padding([6, 12])
            .style(button_style_secondary);

    let mut summary_row =
        row![scheduler_chip, completed_chip].spacing(8).align_y(Alignment::Center);
    if office_selected && app.task_board_worktree_slot_recycle_in_flight {
        summary_row = summary_row.push(status_chip(
            i18n.format("正在回收 worktree%s", &[running_dots(runtime_time::now_ms()).into()])
                .expect("valid action template"),
            Color::from_rgb8(0xFF, 0x7A, 0x00),
        ));
    }

    let controls_row = row![
        view_switch,
        exec_btn,
        new_task_btn,
        settings_btn,
        archive_completed_btn,
        refresh_btn,
        close_btn,
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    let controls_scroll = scrollable(controls_row)
        .direction(iced::widget::scrollable::Direction::Horizontal(
            iced::widget::scrollable::Scrollbar::new().width(4).scroller_width(4),
        ))
        .height(Length::Shrink)
        .width(Length::Fill);

    let top_controls = container(
        column![row![title, summary_row].spacing(12).align_y(Alignment::Center), controls_scroll]
            .spacing(12),
    )
    .padding(Padding::from([16, 18]))
    .style(top_controls_panel_style);

    let mut content = column![top_controls].spacing(12);
    if office_selected && app.project_path.is_some() {
        let office = match app.task_board_worktree_snapshot {
            Some(snapshot) => build_worktree_pixel_office(app, snapshot),
            None => container(
                text(i18n.__("暂无工位数据")).size(12).style(settings_muted_text_style),
            )
            .padding([8, 10])
            .width(Length::Fill)
            .style(worktree_status_panel_style)
            .into(),
        };
        content = content.push(office);
    }

    container(content).padding([18, 20]).width(Length::Fill).style(control_outer_panel_style).into()
}

/// 控制区实际读取的展示参数；宿主负责全局消息包装。
pub struct ControlView<'a> {
    pub desktop_locale: config_ui::DesktopLocale,
    pub window_size: (f32, f32),
    pub project_path: Option<&'a str>,
    pub task_board_tasks: &'a [desktop_task_model::Task],
    pub task_board_executor_running: bool,
    pub task_board_worktree_slot_recycle_in_flight: bool,
    pub task_board_worktree_pixel_office: bool,
    pub task_board_worktree_snapshot: Option<&'a desktop_task_executor::executor::state::WorktreePoolSnapshot>,
    pub agent_entries: Vec<desktop_task_model::agent_activity::AgentEntry<'a>>,
}

#[cfg(test)]
mod test_support;
