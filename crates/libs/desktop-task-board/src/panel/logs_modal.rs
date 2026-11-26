//! 任务看板侧栏面板模块，负责任务编辑、执行器选择和日志弹窗界面。
//!
//! 本模块只负责视图组合与样式适配，不持有业务状态，也不扩大外部能力边界。

use iced::widget::{Space, column, container, row, scrollable, text};
use iced::{Alignment, Color, Element, Length, Theme};

use desktop_ui_controls::system_settings_common::settings_close_button;
use crate::message::TaskBoardMessage;
use desktop_task_model::Task;
#[cfg(test)]
use desktop_task_model::TaskLogEntry;
use crate::message::TaskBoardMessage as Message;

use super::styles::panel_container_style;

/// 构建对应界面片段。
///
/// # 参数
/// - `app`: 当前视图构建所需的状态、配置或消息。
/// - `task`: 当前视图构建所需的状态、配置或消息。
///
/// # 返回
/// 返回可直接嵌入父级视图的 Iced 控件或样式闭包。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
#[allow(dead_code)]
pub fn build_task_logs_modal_content<'a>(
    i18n: desktop_i18n::I18n,
    task: &'a Task,
) -> Element<'a, Message> {
    let close_btn = settings_close_button(TaskBoardMessage::CloseTaskLogs);

    let header = row![
        text(&task.id).size(11).style(task_id_text_style),
        Space::new().width(Length::Fill),
        close_btn,
    ]
    .spacing(8)
    .width(Length::Fill);

    let mut main_col = column![header, Space::new().height(12.0)].spacing(0).width(Length::Fill);


    main_col = build_logs_section(i18n, task, main_col);

    let content = scrollable(main_col).width(Length::Fill).height(Length::Fill);

    container(content)
        .padding(20)
        .style(modal_panel_style)
        .width(Length::FillPortion(1))
        .height(Length::Fill)
        .into()
}

fn task_id_text_style(theme: &Theme) -> iced::widget::text::Style {
    iced::widget::text::Style { color: Some(theme.extended_palette().background.base.text) }
}

fn modal_panel_style(theme: &Theme) -> iced::widget::container::Style {
    let mut style = panel_container_style(theme);
    style.shadow = iced::Shadow {
        color: Color::BLACK.scale_alpha(0.2),
        offset: iced::Vector::new(-4.0, 0.0),
        blur_radius: 16.0,
    };
    style
}




fn build_logs_section<'a>(
    i18n: desktop_i18n::I18n,
    task: &'a Task,
    mut main_col: iced::widget::Column<'a, Message>,
) -> iced::widget::Column<'a, Message> {
    let logs_title = text(i18n.__("任务日志"))
        .size(13)
        .font(iced::Font { weight: iced::font::Weight::Bold, ..Default::default() });
    main_col = main_col.push(logs_title).push(Space::new().height(8.0));

    let logs_content = task
        .logs
        .iter()
        .map(|entry| {
            let timestamp = format!(
                "{}:{} - ",
                entry.timestamp_ms / 60000,
                (entry.timestamp_ms % 60000) / 1000
            );
            format!("{timestamp}{}", desktop_i18n::tasks::log_message(i18n, entry.localized_message.as_ref(), match (entry.status_from, entry.status_to) {
                (Some(from), Some(to)) => Some((
                    desktop_i18n::tasks::status_label(i18n, &from.label_for_workflow_id(""), matches!(from, desktop_task_model::TaskStatus::Custom(_))),
                    desktop_i18n::tasks::status_label(i18n, &to.label_for_workflow_id(""), matches!(to, desktop_task_model::TaskStatus::Custom(_))),
                )),
                _ => None,
            }, &entry.message))
        })
        .collect::<Vec<_>>()
        .join("\n");

    let logs_widget =
        container(text(logs_content).size(11).wrapping(iced::widget::text::Wrapping::Word))
            .padding([8, 10])
            .style(logs_container_style)
            .width(Length::Fill);
    let scrollable_widget = scrollable(logs_widget).height(Length::Fill);
    main_col.push(scrollable_widget)
}

#[cfg(test)]
fn format_logs_content(task: &Task) -> String {
    task.logs.iter().map(format_log_entry).collect::<Vec<_>>().join("\n")
}

#[cfg(test)]
fn format_log_entry(log: &TaskLogEntry) -> String {
    let ts_mins = log.timestamp_ms / 60000;
    let ts_secs = (log.timestamp_ms % 60000) / 1000;
    format!("{}:{} - {}", ts_mins, ts_secs, log.message)
}

fn logs_container_style(theme: &Theme) -> iced::widget::container::Style {
    let p = theme.extended_palette();
    iced::widget::container::Style {
        background: Some(iced::Background::Color(p.background.weak.color.scale_alpha(0.35))),
        border: iced::Border {
            radius: 8.0.into(),
            width: 1.0,
            color: p.background.strong.color.scale_alpha(0.4),
        },
        ..Default::default()
    }
}

#[cfg(test)]
#[path = "logs_modal_tests.rs"]
mod logs_modal_tests;
