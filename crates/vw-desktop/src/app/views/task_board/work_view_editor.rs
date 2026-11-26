//! 工作视图的组合筛选与个人保存操作。
use crate::app::components::system_settings_common::*;
use crate::app::message::TaskBoardMessage;
use crate::app::{App, Message};
use iced::widget::{button, checkbox, column, container, row, scrollable, text, text_input};
use iced::{Element, Length};

pub fn view(app: &App) -> Element<'_, Message> {
    let board = &app.task_board;
    let i18n = crate::app::i18n::I18n::new(app.general_settings.desktop_locale);
    let action = |label: &'static str, message| {
        button(text(i18n.__(label)).size(13))
            .padding([7, 10])
            .style(rounded_action_btn_style)
            .on_press(Message::TaskBoard(message))
    };
    let priority = |value: Option<u32>| {
        value.map_or_else(|| i18n.__("不限").to_owned(), |value| value.to_string())
    };
    let agents = board.work_filter.agents.iter().map(|agent| match agent {
        desktop_task_board::work_views::ExecutionAgent::Builtin { id } => format!("builtin:{id}"),
        desktop_task_board::work_views::ExecutionAgent::Acp { id } => format!("acp:{id}"),
    }).collect::<Vec<_>>().join(", ");
    let summary = i18n.format(
        "状态 [%s] · 优先级 %s 至 %s · 智能体 [%s]",
        &[
            board.work_filter.status_keys.join(", ").into(),
            priority(board.work_filter.priority_min).into(),
            priority(board.work_filter.priority_max).into(),
            agents.into(),
        ],
    ).expect("valid work view summary template");
    let mut actions = row![].spacing(8);
    if board.work_filter != desktop_task_board::work_views::WorkViewFilter::default() {
        actions = actions.push(action(i18n.__("清除条件"), TaskBoardMessage::ClearWorkFilter));
    }
    let mut content = column![actions, text(summary).size(12)].spacing(8);
    if !board.work_filter.label_ids.is_empty() {
        let labels = board.task_labels.iter()
            .filter(|label| board.work_filter.label_ids.contains(&label.id));
        content = content.push(
            row(labels.map(desktop_task_board::board::task_label_badge)).spacing(4).wrap()
        );
    }
    if !board.work_filter.properties.is_empty() {
        content = content.push(super::properties::filter_summary(app));
    }
    if board.work_editor_open {
        let field = |label: &'static str, value: &str, index| {
            text_input(i18n.__(label), value)
                .on_input(move |value| {
                    Message::TaskBoard(TaskBoardMessage::WorkViewFieldChanged(index, value))
                })
                .style(settings_text_input_style)
                .padding(SETTINGS_CONTROL_PADDING)
                .size(SETTINGS_CONTROL_TEXT_SIZE)
        };
        let mut fields = column![
            text(i18n.__("多项状态或智能体用英文逗号分隔。若选中多个标签，匹配任一即可。所有属性条件须同时满足。")).size(12),
            field(i18n.__("状态 key（空值不限）"), &board.work_status_input, 1),
            field(i18n.__("最低优先级"), &board.work_min_input, 2),
            field(i18n.__("最高优先级"), &board.work_max_input, 3),
            field(i18n.__("智能体：builtin:标识, acp:标识"), &board.work_agent_input, 4),
            field(i18n.__("保存视图名称（更新名称即可重命名）"), &board.work_view_name, 0),
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
        if !board.task_properties.is_empty() || !board.work_property_values.is_empty() {
            fields = fields.push(settings_field_label(i18n.__("属性")))
                .push(super::properties::filter_editor(app));
        }
        let mut actions = row![
            action(i18n.__("应用筛选"), TaskBoardMessage::ApplyWorkFilter),
            button(text(i18n.__("另存视图")).size(13))
                .padding([7, 10])
                .style(primary_action_btn_style)
                .on_press(Message::TaskBoard(TaskBoardMessage::SaveWorkView(true))),
        ]
        .spacing(8);
        if board.selected_work_view.is_some() {
            actions = actions
                .push(action(i18n.__("更新 / 重命名"), TaskBoardMessage::SaveWorkView(false)))
                .push(action(i18n.__("删除视图"), TaskBoardMessage::DeleteWorkView));
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
