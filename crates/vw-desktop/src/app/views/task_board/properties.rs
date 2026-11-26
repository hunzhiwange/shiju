//! 类型化任务属性控件，由任务编辑和工作视图共用。

use crate::app::components::system_settings::task_settings::{catalog, properties::property_icon};
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

fn find_option<'a>(property: Option<&'a Property>, id: &str) -> Option<&'a PropertyOption> {
    property?.options.iter().find(|option| option.id == id)
}

fn selected_label(i18n: I18n, property: Option<&Property>, id: &str) -> String {
    find_option(property, id)
        .map_or_else(|| i18n.__("选项已不存在").into(), |option| option_label(i18n, option))
}

fn value_label(i18n: I18n, property: Option<&Property>, value: &Value) -> String {
    match value {
        Value::SingleSelect(id) => selected_label(i18n, property, id),
        Value::MultiSelect(ids) => {
            ids.iter().map(|id| selected_label(i18n, property, id)).collect::<Vec<_>>().join(" · ")
        }
        Value::Checkbox(checked) => i18n.__(if *checked { "已勾选" } else { "未勾选" }).into(),
        Value::Date(raw)
        | Value::Text(raw)
        | Value::Number(raw)
        | Value::Link(raw)
        | Value::Member(raw) => raw.clone(),
        Value::MultiMember(items) | Value::TextList(items) | Value::LinkList(items) => {
            items.join(" · ")
        }
    }
}

pub(super) fn task_summary<'a>(
    app: &'a App,
    task: &'a desktop_task_model::Task,
) -> impl Iterator<Item = (&'a str, String)> + 'a {
    let i18n = I18n::new(app.general_settings.desktop_locale);
    app.task_board.task_properties.iter().filter_map(move |property| {
        let value = task.properties.get(&property.id)?;
        Some((property.name.as_str(), value_label(i18n, Some(property), value)))
    })
}

fn option_color(property: &Property, id: &str) -> Option<iced::Color> {
    find_option(Some(property), id)?.color.as_deref().and_then(catalog::palette_color)
}

fn choice_control<'a>(
    i18n: I18n,
    property: &'a Property,
    value: Option<&'a Value>,
    enabled: bool,
    include_archived_options: bool,
    change: impl Fn(Option<Value>) -> TaskBoardMessage + 'a,
) -> Element<'a, Message> {
    let multi = match value {
        Some(Value::MultiSelect(ids)) => ids.as_slice(),
        _ => &[],
    };
    let choices = property
        .options
        .iter()
        .filter(|option| !option.archived || include_archived_options)
        .filter(|option| !multi.contains(&option.id))
        .map(|option| Choice { id: option.id.clone(), name: option_label(i18n, option) })
        .collect::<Vec<_>>();
    let selected = match value {
        Some(Value::SingleSelect(id)) => {
            Some(Choice { id: id.clone(), name: selected_label(i18n, Some(property), id) })
        }
        _ => None,
    };
    let selected_color = selected.as_ref().and_then(|choice| option_color(property, &choice.id));
    let placeholder = if choices.is_empty() {
        i18n.__("此属性没有可用选项")
    } else if property.kind == Kind::MultiSelect {
        i18n.__("添加选项")
    } else {
        i18n.__("请选择")
    };
    let change = std::rc::Rc::new(change);
    let on_select = change.clone();
    let picker = settings_select(choices, selected, move |choice| {
        let value = if property.kind == Kind::MultiSelect {
            let mut next = multi.to_vec();
            next.push(choice.id);
            next.sort_unstable();
            Value::MultiSelect(next)
        } else {
            Value::SingleSelect(choice.id)
        };
        Message::TaskBoard(on_select(Some(value)))
    })
    .placeholder(placeholder)
    .width(Length::Fill)
    .disabled(!enabled);
    let mut control = row![].spacing(8).align_y(Alignment::Center).width(Length::Fill);
    if let Some(color) = selected_color {
        control = control.push(catalog::color_dot(color, 10.0));
    }
    control = control.push(picker);
    if multi.is_empty() {
        return control.into();
    }
    let chips = multi.iter().enumerate().map(|(index, id)| {
        let mut chip = row![].spacing(5).align_y(Alignment::Center);
        if let Some(color) = option_color(property, id) {
            chip = chip.push(catalog::color_dot(color, 8.0));
        }
        chip = chip
            .push(text(selected_label(i18n, Some(property), id)).size(12))
            .push(catalog::glyph(desktop_assets::assets::Icon::X, 11.0));
        let remove = enabled.then(|| {
            let mut next = multi.to_vec();
            next.remove(index);
            Message::TaskBoard(change((!next.is_empty()).then_some(Value::MultiSelect(next))))
        });
        button(chip).padding([5, 8]).style(rounded_action_btn_style).on_press_maybe(remove).into()
    });
    column![control, row(chips).spacing(5).wrap()].spacing(6).into()
}

fn list_control<'a>(
    i18n: I18n,
    kind: Kind,
    value: Option<&'a Value>,
    enabled: bool,
    change: impl Fn(Option<Value>) -> TaskBoardMessage + 'a,
) -> Element<'a, Message> {
    let items = match value {
        Some(Value::TextList(items) | Value::LinkList(items) | Value::MultiMember(items)) => {
            items.as_slice()
        }
        _ => &[],
    };
    let (placeholder, wrap): (_, fn(Vec<String>) -> Value) = match kind {
        Kind::LinkList => ("https://…", Value::LinkList),
        Kind::MultiMember => (i18n.__("输入成员 ID"), Value::MultiMember),
        _ => (i18n.__("输入文本"), Value::TextList),
    };
    let change = std::rc::Rc::new(move |items: Vec<String>| {
        Message::TaskBoard(change((!items.is_empty()).then(|| wrap(items))))
    });
    let mut fields = column![].width(Length::Fill).spacing(5);
    for (index, item) in items.iter().enumerate() {
        let input = settings_text_input(placeholder, item).width(Length::Fill);
        let input = if enabled {
            let change = change.clone();
            input.on_input(move |text| {
                let mut next = items.to_vec();
                next[index] = text;
                change(next)
            })
        } else {
            input
        };
        let remove = enabled.then(|| {
            let mut next = items.to_vec();
            next.remove(index);
            change(next)
        });
        fields = fields.push(
            row![
                input,
                button(catalog::glyph(desktop_assets::assets::Icon::X, 13.0))
                    .padding(7)
                    .style(rounded_action_btn_style)
                    .on_press_maybe(remove),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        );
    }
    let add = (enabled && items.iter().all(|item| !item.trim().is_empty())).then(|| {
        let mut next = items.to_vec();
        next.push(String::new());
        change(next)
    });
    fields
        .push(
            button(text(i18n.__("添加条目")).size(12))
                .padding([5, 9])
                .style(rounded_action_btn_style)
                .on_press_maybe(add),
        )
        .into()
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
        Kind::SingleSelect | Kind::MultiSelect => {
            choice_control(i18n, property, value, enabled, include_archived_options, change)
        }
        Kind::Checkbox => {
            let selected = match value {
                Some(Value::Checkbox(checked)) => Some(*checked),
                _ => None,
            };
            settings_select([true, false], selected, move |checked| {
                Message::TaskBoard(change(Some(Value::Checkbox(checked))))
            })
            .format_labels(move |checked| {
                i18n.__(if *checked { "已勾选" } else { "未勾选" }).to_owned()
            })
            .placeholder(i18n.__("未设置"))
            .width(Length::Fill)
            .disabled(!enabled)
            .into()
        }
        Kind::TextList | Kind::LinkList | Kind::MultiMember => {
            list_control(i18n, property.kind, value, enabled, change)
        }
        kind => {
            let raw = match value {
                Some(
                    Value::Date(raw)
                    | Value::Text(raw)
                    | Value::Number(raw)
                    | Value::Link(raw)
                    | Value::Member(raw),
                ) => raw.as_str(),
                _ => "",
            };
            let (placeholder, wrap): (_, fn(String) -> Value) = match kind {
                Kind::Date => ("YYYY-MM-DD", Value::Date),
                Kind::Number => (i18n.__("输入数字"), Value::Number),
                Kind::Link => ("https://…", Value::Link),
                Kind::Member => (i18n.__("输入成员 ID"), Value::Member),
                _ => (i18n.__("输入文本"), Value::Text),
            };
            let input = settings_text_input(placeholder, raw);
            if enabled {
                input
                    .on_input(move |value| {
                        Message::TaskBoard(change((!value.is_empty()).then(|| wrap(value))))
                    })
                    .into()
            } else {
                input.into()
            }
        }
    }
}

fn clear<'a>(
    i18n: I18n,
    message: TaskBoardMessage,
    enabled: bool,
) -> iced::widget::Button<'a, Message> {
    button(text(i18n.__("清空")).size(12))
        .padding([6, 10])
        .style(rounded_action_btn_style)
        .on_press_maybe(enabled.then_some(Message::TaskBoard(message)))
}


pub(super) fn task_editor<'a>(
    app: &'a App,
    task: &'a desktop_task_model::Task,
) -> Option<Element<'a, Message>> {
    let board = &app.task_board;
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let enabled = board.task_property_save_in_progress.is_none();
    if !board
        .task_properties
        .iter()
        .any(|property| !property.archived || task.properties.contains_key(&property.id))
    {
        return None;
    }
    let mut fields = column![].spacing(8);
    for property in &board.task_properties {
        let stored = task.properties.get(&property.id);
        if property.archived {
            if let Some(stored) = stored {
                fields = fields.push(
                    column![
                        settings_field_label(&property.name),
                        text(format!(
                            "{} · {}",
                            value_label(i18n, Some(property), stored),
                            i18n.__("已归档")
                        ))
                        .size(12)
                        .style(settings_muted_text_style),
                    ]
                    .spacing(4),
                );
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
        fields = fields.push(
            column![
                row![
                    catalog::glyph(property_icon(property.icon), 14.0),
                    settings_field_label(&property.name)
                ]
                .spacing(6)
                .align_y(Alignment::Center),
                row![
                    control(i18n, property, value, enabled, false, change),
                    clear(i18n, clear_message, enabled && value.is_some()),
                    save,
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            ]
            .spacing(4),
        );
    }
    Some(
        column![
            settings_field_label(i18n.__("属性")),
            container(
                scrollable(container(fields).padding([0, 7])).height(Length::Shrink).direction(
                    scrollable::Direction::Vertical(
                        scrollable::Scrollbar::new().width(4).scroller_width(4),
                    )
                )
            )
            .max_height(160),
        ]
        .spacing(6)
        .into(),
    )
}

pub(super) fn filter_editor(app: &App) -> Element<'_, Message> {
    let board = &app.task_board;
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let mut fields = column![].spacing(8);
    for property in &board.task_properties {
        let value = board.work_property_values.get(&property.id);
        let change =
            move |value| TaskBoardMessage::WorkViewPropertyChanged(property.id.clone(), value);
        let clear_message = change(None);
        let name = if property.archived {
            format!("{} · {}", property.name, i18n.__("已归档"))
        } else {
            property.name.clone()
        };
        fields = fields.push(
            column![
                text(name).size(13),
                row![
                    control(i18n, property, value, true, true, change),
                    clear(i18n, clear_message, value.is_some()),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            ]
            .spacing(4),
        );
    }
    for id in board.work_property_values.keys() {
        if !board.task_properties.iter().any(|property| &property.id == id) {
            fields = fields.push(
                row![
                    text(i18n.__("属性已不存在")).size(13).style(settings_muted_text_style),
                    clear(i18n, TaskBoardMessage::WorkViewPropertyChanged(id.clone(), None), true),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        }
    }
    fields.into()
}

pub(super) fn filter_summary(app: &App) -> Element<'_, Message> {
    let board = &app.task_board;
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let filters = board.work_filter.properties.iter().map(|(id, value)| {
        let property = board.task_properties.iter().find(|property| &property.id == id);
        let name =
            property.map_or(i18n.__("属性已不存在"), |property| property.name.as_str());
        text(format!("{name} · {}", value_label(i18n, property, value))).size(12).into()
    });
    row(filters).spacing(8).wrap().into()
}
