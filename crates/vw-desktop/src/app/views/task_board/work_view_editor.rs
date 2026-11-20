//! 工作视图的组合筛选与个人保存操作。
use crate::app::components::system_settings_common::*;
use crate::app::message::TaskBoardMessage;
use crate::app::{App, Message};
use iced::widget::{button, checkbox, column, container, row, scrollable, text, text_input};
use iced::{Element, Length};

pub fn view(app: &App) -> Element<'_, Message> {
    let board = &app.task_board;
    let action = |label: &'static str, message| {
        button(text(label).size(13))
            .padding([7, 10])
            .style(rounded_action_btn_style)
            .on_press(Message::TaskBoard(message))
    };
    let mut content = column![
        row![
            action("添加筛选 / 收起", TaskBoardMessage::OpenWorkViewEditor),
            action("清除条件", TaskBoardMessage::ClearWorkFilter),
        ]
        .spacing(8),
        text(format!(
            "已启用：状态 [{}] · 优先级 {} 至 {} · 智能体 [{}]",
            board.work_filter.status_keys.join(", "),
            board.work_filter.priority_min.map(|n| n.to_string()).unwrap_or_else(|| "不限".into()),
            board.work_filter.priority_max.map(|n| n.to_string()).unwrap_or_else(|| "不限".into()),
            board
                .work_filter
                .agents
                .iter()
                .map(|a| match a {
                    desktop_task_board::work_views::ExecutionAgent::Builtin { id } =>
                        format!("builtin:{id}"),
                    desktop_task_board::work_views::ExecutionAgent::Acp { id } =>
                        format!("acp:{id}"),
                })
                .collect::<Vec<_>>()
                .join(", ")
        ))
        .size(12),
    ]
    .spacing(8);
    if !board.work_filter.label_ids.is_empty() {
        let labels = board.task_labels.iter()
            .filter(|label| board.work_filter.label_ids.contains(&label.id));
        content = content.push(
            row(labels.map(desktop_task_board::board::task_label_badge)).spacing(4).wrap()
        );
    }
    if board.work_editor_open {
        let field = |label: &'static str, value: &str, index| {
            text_input(label, value)
                .on_input(move |value| {
                    Message::TaskBoard(TaskBoardMessage::WorkViewFieldChanged(index, value))
                })
                .style(settings_text_input_style)
                .padding(SETTINGS_CONTROL_PADDING)
                .size(SETTINGS_CONTROL_TEXT_SIZE)
        };
        let i18n = crate::app::i18n::I18n::new(app.general_settings.desktop_locale);
        let mut fields = column![
            text("同字段多项用英文逗号分隔；不同字段同时满足").size(12),
            field("状态 key（空值不限）", &board.work_status_input, 1),
            field("最低优先级", &board.work_min_input, 2),
            field("最高优先级", &board.work_max_input, 3),
            field("智能体：builtin:标识, acp:标识", &board.work_agent_input, 4),
            field("保存视图名称（更新名称即可重命名）", &board.work_view_name, 0),
        ]
        .spacing(8);
        let choices = board.task_labels.iter().filter_map(|label| {
            let selected = board.work_label_ids.contains(&label.id);
            if label.archived && !selected {
                return None;
            }
            let id = label.id.clone();
            Some(
                checkbox(selected)
                    .label(&label.name)
                    .style(settings_checkbox_style)
                    .on_toggle(move |_| Message::TaskBoard(TaskBoardMessage::ToggleWorkViewLabel(id.clone())))
                    .into(),
            )
        }).collect::<Vec<Element<'_, Message>>>();
        if !choices.is_empty() {
            fields = fields.push(settings_field_label(i18n.__("标签")))
                .push(row(choices).spacing(8).wrap());
        }
        let mut actions = row![
            action("应用筛选", TaskBoardMessage::ApplyWorkFilter),
            button(text("另存视图").size(13))
                .padding([7, 10])
                .style(primary_action_btn_style)
                .on_press(Message::TaskBoard(TaskBoardMessage::SaveWorkView(true))),
        ]
        .spacing(8);
        if board.selected_work_view.is_some() {
            actions = actions
                .push(action("更新 / 重命名", TaskBoardMessage::SaveWorkView(false)))
                .push(action("删除视图", TaskBoardMessage::DeleteWorkView));
        }
        content = content
            .push(scrollable(container(fields).padding([0, 7])).height(210).direction(
                scrollable::Direction::Vertical(
                    scrollable::Scrollbar::new().width(4).scroller_width(4),
                ),
            ))
            .push(actions.wrap());
    }
    container(content).width(Length::Fill).padding(8).into()
}
