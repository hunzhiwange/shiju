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
                (true, true) => i18n.__("此属性没有可用选项"),
                (true, false) => i18n.__("全部选项已归档"),
                _ => i18n.__("请选择"),
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
                (_, Some(Value::Text(raw))) => (i18n.__("输入文本"), raw.as_str()),
                _ => (i18n.__("输入文本"), ""),
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

fn batch_editor<'a>(i18n: desktop_i18n::I18n, app: &'a App, task: &desktop_task_model::Task) -> Option<Element<'a, Message>> {
    use vw_api_agent_scheduler::AgentSchedulerTaskPlanPlacementDto as Placement;

    let (_, plan) = super::work_plan::current(app)?;
    let snapshot = plan.tasks.get(&task.id)?;
    if task.archived || task.deleted {
        return None;
    }
    let stored = match &snapshot.placement {
        Some(Placement::Child { .. }) => return None,
        Some(Placement::Root { outer_batch, inner_batch }) => Some(format!("{outer_batch}.{inner_batch}")),
        None => None,
    };
    let draft = plan.batch_draft.as_ref().filter(|draft| draft.task_id == task.id);
    let pending = draft.is_some_and(|draft| draft.pending.is_some()) || plan.pending_batches.contains_key(&task.id);
    let header = row![
        text(stored.as_ref().map_or_else(|| i18n.__("无批次").into(), |batch| i18n.format("已保存批次 %s", &[format!("{}", batch).into()]).expect("valid workbench template"))).size(13),
        button(text(if draft.is_some() { i18n.__("重新载入批次") } else { i18n.__("安排批次") }).size(12))
            .padding([6, 10]).style(rounded_action_btn_style)
            .on_press_maybe((!pending).then_some(Message::TaskBoard(TaskBoardMessage::EditWorkPlanBatch))),
    ].spacing(8).align_y(Alignment::Center);
    let mut body = column![header].spacing(6);
    if let Some(draft) = draft {
        let mut outer = settings_text_input(i18n.__("外层批次"), &draft.outer).id(super::work_plan::OUTER_INPUT);
        let mut inner = settings_text_input(i18n.__("组内批次"), &draft.inner);
        let save = Message::TaskBoard(TaskBoardMessage::SaveWorkPlanBatch { clear: false });
        if !pending {
            outer = outer.on_input(|value| Message::TaskBoard(TaskBoardMessage::WorkPlanBatchChanged { outer: true, value }));
            inner = inner.on_input(|value| Message::TaskBoard(TaskBoardMessage::WorkPlanBatchChanged { outer: false, value }))
                .on_submit(save.clone());
        }
        let feedback = if pending {
            i18n.__("正在提交批次…").to_owned()
        } else {
            match &draft.feedback {
                None => i18n.__("批次草稿").to_owned(),
                Some(Ok(())) => i18n.__("批次已保存").to_owned(),
                Some(Err(error)) => error.render(i18n),
            }
        };
        body = body.push(row![
            column![settings_field_label(i18n.__("外层批次")), outer].spacing(4),
            column![settings_field_label(i18n.__("组内批次")), inner].spacing(4),
        ].spacing(8))
        .push(row![
            button(text(i18n.__("保存批次")).size(12)).padding([6, 10]).style(primary_action_btn_style)
                .on_press_maybe((!pending).then_some(save)),
            button(text(i18n.__("清空批次")).size(12)).padding([6, 10]).style(rounded_action_btn_style)
                .on_press_maybe((!pending && (stored.is_some() || !draft.outer.is_empty() || !draft.inner.is_empty()))
                    .then_some(Message::TaskBoard(TaskBoardMessage::SaveWorkPlanBatch { clear: true }))),
            text(feedback).size(12),
        ].spacing(8).wrap());
    } else if pending {
        body = body.push(text(i18n.__("正在提交批次…")).size(12));
    }
    Some(body.into())
}

pub(super) fn task_editor<'a>(app: &'a App, task: &'a desktop_task_model::Task) -> Option<Element<'a, Message>> {
    let board = &app.task_board;
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let batch = batch_editor(i18n, app, task);
    let enabled = board.task_property_save_in_progress.is_none();
    if !board.task_properties.iter().any(|property| {
        !property.archived || task.properties.contains_key(&property.id)
    }) {
        return batch.map(|batch| column![settings_field_label(i18n.__("属性")), batch].spacing(4).into());
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
    ].extend(batch).spacing(6).into())
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
