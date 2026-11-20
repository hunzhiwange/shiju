//! 类型化任务属性控件，由任务编辑和工作视图共用。

use crate::app::components::system_settings_common::*;
use crate::app::i18n::I18n;
use crate::app::message::TaskBoardMessage;
use crate::app::{App, Message};
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Alignment, Element, Length};
use vw_api_agent_scheduler::{
    AgentSchedulerTaskPropertyDto as Property, AgentSchedulerTaskPropertyKind as Kind,
    AgentSchedulerTaskPropertyOption as PropertyOption, AgentSchedulerTaskPropertyValue as Value,
};

#[derive(Clone, PartialEq)]
struct Choice {
    id: String,
    name: String,
}

impl std::fmt::Display for Choice {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.name)
    }
}

fn option_label(i18n: I18n, option: &PropertyOption) -> String {
    if option.archived {
        format!("{} · {}", option.name, i18n.__("选项已归档"))
    } else {
        option.name.clone()
    }
}

fn value_label(i18n: I18n, property: Option<&Property>, value: &Value) -> String {
    match value {
        Value::SingleSelect(id) => property
            .and_then(|property| property.options.iter().find(|option| &option.id == id))
            .map_or_else(|| i18n.__("选项已不存在").into(), |option| option_label(i18n, option)),
        Value::Date(date) | Value::Text(date) => date.clone(),
    }
}

fn control<'a>(
    i18n: I18n,
    property: &'a Property,
    value: Option<&'a Value>,
    enabled: bool,
    include_archived_options: bool,
    change: impl Fn(Option<Value>) -> TaskBoardMessage + 'a,
) -> Element<'a, Message> {
    match property.kind {
        Kind::SingleSelect => {
            let choices = property.options.iter()
                .filter(|option| !option.archived || include_archived_options)
                .map(|option| Choice {
                    id: option.id.clone(),
                    name: option_label(i18n, option),
                })
                .collect::<Vec<_>>();
            let selected = match value {
                Some(Value::SingleSelect(id)) => Some(
                    choices.iter().find(|choice| &choice.id == id).cloned().unwrap_or_else(|| Choice {
                        id: id.clone(),
                        name: value_label(i18n, Some(property), &Value::SingleSelect(id.clone())),
                    }),
                ),
                _ => None,
            };
            let placeholder = match (choices.is_empty(), property.options.is_empty()) {
                (true, true) => "此属性没有可用选项",
                (true, false) => "全部选项已归档",
                _ => "请选择",
            };
            settings_select(choices, selected, move |choice| {
                Message::TaskBoard(change(Some(Value::SingleSelect(choice.id))))
            })
            .placeholder(i18n.__(placeholder))
            .width(Length::Fill)
            .disabled(!enabled)
            .into()
        }
        Kind::Date | Kind::Text => {
            let (placeholder, raw) = match (property.kind, value) {
                (Kind::Date, Some(Value::Date(raw))) => ("YYYY-MM-DD", raw.as_str()),
                (Kind::Date, _) => ("YYYY-MM-DD", ""),
                (_, Some(Value::Text(raw))) => ("输入文本", raw.as_str()),
                _ => ("输入文本", ""),
            };
            let input = settings_text_input(i18n.__(placeholder), raw);
            if enabled {
                input.on_input(move |value| {
                    let value = (!value.is_empty()).then(|| match property.kind {
                        Kind::Date => Value::Date(value),
                        _ => Value::Text(value),
                    });
                    Message::TaskBoard(change(value))
                }).into()
            } else {
                input.into()
            }
        }
    }
}

fn clear<'a>(i18n: I18n, message: TaskBoardMessage, enabled: bool) -> iced::widget::Button<'a, Message> {
    button(text(i18n.__("清空")).size(12))
        .padding([6, 10])
        .style(rounded_action_btn_style)
        .on_press_maybe(enabled.then_some(Message::TaskBoard(message)))
}

pub(super) fn task_editor<'a>(app: &'a App, task: &'a desktop_task_model::Task) -> Option<Element<'a, Message>> {
    let board = &app.task_board;
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let enabled = board.task_property_save_in_progress.is_none();
    if !board.task_properties.iter().any(|property| {
        !property.archived || task.properties.contains_key(&property.id)
    }) {
        return None;
    }
    let mut fields = column![].spacing(8);
    for property in &board.task_properties {
        let stored = task.properties.get(&property.id);
        if property.archived {
            if let Some(stored) = stored {
                fields = fields.push(column![
                    settings_field_label(&property.name),
                    text(format!("{} · {}", value_label(i18n, Some(property), stored), i18n.__("已归档")))
                        .size(12).style(settings_muted_text_style),
                ].spacing(4));
            }
            continue;
        }
        let pending = board.task_board_draft.property_changes.get(&property.id);
        let value = pending.map_or(stored, Option::as_ref);
        let change = move |value| TaskBoardMessage::EditingTaskPropertyChanged {
            task_id: task.id.clone(),
            field_id: property.id.clone(),
            value,
        };
        let clear_message = change(None);
        let save = button(text(i18n.__("保存")).size(12))
            .padding([6, 10])
            .style(primary_action_btn_style)
            .on_press_maybe((enabled && pending.is_some() && value != stored).then(|| {
                Message::TaskBoard(TaskBoardMessage::SaveEditingTaskProperty {
                    task_id: task.id.clone(),
                    field_id: property.id.clone(),
                })
            }));
        fields = fields.push(column![
            settings_field_label(&property.name),
            row![
                control(i18n, property, value, enabled, false, change),
                clear(i18n, clear_message, enabled && value.is_some()),
                save,
            ].spacing(8).align_y(Alignment::Center),
        ].spacing(4));
    }
    Some(column![
        settings_field_label(i18n.__("属性")),
        container(
            scrollable(container(fields).padding([0, 7]))
                .height(Length::Shrink)
                .direction(scrollable::Direction::Vertical(
                    scrollable::Scrollbar::new().width(4).scroller_width(4),
                ))
        ).max_height(160),
    ].spacing(4).into())
}

pub(super) fn filter_editor(app: &App) -> Element<'_, Message> {
    let board = &app.task_board;
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let mut fields = column![].spacing(8);
    for property in &board.task_properties {
        let value = board.work_property_values.get(&property.id);
        let change = move |value| TaskBoardMessage::WorkViewPropertyChanged(property.id.clone(), value);
        let clear_message = change(None);
        let name = if property.archived {
            format!("{} · {}", property.name, i18n.__("已归档"))
        } else {
            property.name.clone()
        };
        fields = fields.push(column![
            text(name).size(13),
            row![
                control(i18n, property, value, true, true, change),
                clear(i18n, clear_message, value.is_some()),
            ].spacing(8).align_y(Alignment::Center),
        ].spacing(4));
    }
    for id in board.work_property_values.keys() {
        if !board.task_properties.iter().any(|property| &property.id == id) {
            fields = fields.push(row![
                text(i18n.__("属性已不存在")).size(13).style(settings_muted_text_style),
                clear(i18n, TaskBoardMessage::WorkViewPropertyChanged(id.clone(), None), true),
            ].spacing(8).align_y(Alignment::Center));
        }
    }
    fields.into()
}

pub(super) fn filter_summary(app: &App) -> Element<'_, Message> {
    let board = &app.task_board;
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let filters = board.work_filter.properties.iter().map(|(id, value)| {
        let property = board.task_properties.iter().find(|property| &property.id == id);
        let name = property.map_or(i18n.__("属性已不存在"), |property| property.name.as_str());
        text(format!("{name} · {}", value_label(i18n, property, value))).size(12).into()
    });
    row(filters).spacing(8).wrap().into()
}
