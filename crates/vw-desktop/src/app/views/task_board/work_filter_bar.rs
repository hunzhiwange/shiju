use crate::app::components::system_settings_common::{
    icon_svg, primary_action_btn_style, rounded_action_btn_style, settings_muted_text_style,
};
use crate::app::i18n::I18n;
use crate::app::message::TaskBoardMessage;
use crate::app::{App, Message};
use desktop_assets::assets::Icon;
use desktop_task_board::message::AdvancedWorkFilterAction;
use desktop_task_board::board::task_status_marker;
use desktop_task_board::work_views::{
    ExecutionAgent, SavedWorkView, WorkFilterField, WorkViewFilter,
};
use desktop_task_model::{DEFAULT_TASK_WORKFLOW_ID, TaskStatus};
use desktop_ui_controls::{
    single_line_label::SingleLineLabel, system_settings_common::icons::icon_btn_svg_style,
    tooltip::tooltip_dark_style,
};
use iced::widget::{button, container, row, text, tooltip};
use iced::{Alignment, Element, Length};

use super::work_filter_picker::{agent_choices, field_label, priority_label, priority_marker};

pub(super) fn view<'a>(
    app: &'a App,
    i18n: I18n,
    saved: Option<&SavedWorkView>,
    add: Element<'a, Message>,
) -> Element<'a, Message> {
    let filter = &app.task_board.work_filter;
    let any = *filter != WorkViewFilter::default();
    let mut filters = row![add].spacing(6).align_y(Alignment::Center);
    if any {
        filters = filters.extend(chips(app, i18n, filter));
    } else if saved.is_some() {
        filters = filters
            .push(text(i18n.__("当前显示全部任务")).size(12).style(settings_muted_text_style));
    }
    let mut actions = row![].spacing(6).align_y(Alignment::Center);
    if any {
        actions = actions.push(button(text(i18n.__("清除")).size(13))
            .padding([6, 9])
            .style(rounded_action_btn_style)
            .on_press(TaskBoardMessage::ClearWorkFilter.into()));
    }
    if let Some(saved) = saved {
        actions = actions.push(button(text(i18n.__("保存视图")).size(13))
            .padding([6, 12])
            .style(primary_action_btn_style)
            .on_press_maybe(
                (saved.filter != *filter).then(|| TaskBoardMessage::SaveAppliedWorkFilter.into()),
            ));
    }
    container(
        row![
            container(filters.wrap().vertical_spacing(6)).width(Length::Fill),
            actions,
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .padding([7, 8])
    .style(super::style::filter_bar_surface)
    .into()
}

fn chips(app: &App, i18n: I18n, filter: &WorkViewFilter) -> Vec<Element<'static, Message>> {
    let mut fields = super::work_filter_picker::fields(app);
    let properties = filter.properties.keys().chain(filter.unset_properties.iter())
        .collect::<std::collections::BTreeSet<_>>();
    for id in properties {
        let field = WorkFilterField::Property(id.clone());
        if !fields.iter().any(|(existing, _)| existing == &field) {
            fields.push((field, Icon::Sliders));
        }
    }
    fields.push((WorkFilterField::Advanced, Icon::Sliders));

    let workflow = app.task_board.task_board_settings.workflow()
        .or_else(|| desktop_task_model::task_workflow_by_id(DEFAULT_TASK_WORKFLOW_ID));
    fields.into_iter().filter_map(|(field, icon)| {
        let label = value_summary(app, i18n, &field, filter)?;
        let value = match &field {
            WorkFilterField::Status => {
                let markers = filter.status_keys.iter().filter_map(|key| {
                    workflow.as_ref()?.statuses.iter().find(|status| status.key() == key)
                }).take(3).map(|status| task_status_marker(status.status));
                row(markers).push(value_text(label)).spacing(4).align_y(Alignment::Center).into()
            }
            WorkFilterField::Priority => row(filter.priority_levels.iter().take(3).copied().map(priority_marker))
                .push(value_text(label)).spacing(4).align_y(Alignment::Center).into(),
            WorkFilterField::Date => {
                let range = filter.date.map_or(label, |date| super::work_filter_date::range_label(i18n, date.range));
                container(SingleLineLabel::new(range, 13.0).shrink()).max_width(220).into()
            }
            _ => value_text(label),
        };
        Some(chip(app, i18n, field, icon, value))
    }).collect()
}

pub(super) fn value_summary(
    app: &App,
    i18n: I18n,
    field: &WorkFilterField,
    filter: &WorkViewFilter,
) -> Option<String> {
    let excluded_label = |value: String, excluded| {
        if excluded { format!("{} {value}", i18n.__("排除")) } else { value }
    };
    match field {
        WorkFilterField::Status => {
            let label = match filter.status_keys.as_slice() {
                [] => return None,
                [key] => app.task_board.task_board_settings.workflow()
                    .or_else(|| desktop_task_model::task_workflow_by_id(DEFAULT_TASK_WORKFLOW_ID))
                    .and_then(|workflow| workflow.statuses.into_iter().find(|status| status.key() == key))
                    .map_or_else(|| key.clone(), |status| desktop_i18n::tasks::status_label(
                        i18n, &status.label, matches!(status.status, TaskStatus::Custom(_)),
                    )),
                keys => count(i18n, "%u 个状态", keys.len()),
            };
            Some(excluded_label(label, filter.status_excluded))
        }
        WorkFilterField::AgentRunning => filter.agent_running
            .map(|running| i18n.__(if running { "运行中" } else { "空闲" }).to_owned()),
        WorkFilterField::ReadState => filter.unread_only.then(|| i18n.__("未读").to_owned()),
        WorkFilterField::Priority => {
            let range = match (filter.priority_min, filter.priority_max) {
                (Some(min), Some(max)) if min == max => Some(min.to_string()),
                (Some(min), Some(max)) => Some(format!("{min}–{max}")),
                (Some(min), None) => Some(format!("≥ {min}")),
                (None, Some(max)) => Some(format!("≤ {max}")),
                (None, None) => None,
            };
            if filter.priority_levels.is_empty() {
                return range;
            }
            let label = priority_label(i18n, &filter.priority_levels, filter.priority_excluded);
            Some(range.map_or_else(|| label.clone(), |range| format!("{label} · {range}")))
        }
        WorkFilterField::Date => filter.date.map(|date| {
            let range = super::work_filter_date::range_label(i18n, date.range);
            format!("{} · {range}", i18n.__(date.field.label()))
        }),
        WorkFilterField::Scope(scope) => filter.scopes.get(scope)
            .filter(|selection| !selection.values.is_empty())
            .map(|selection| super::work_filter_scope::summary(app, i18n, *scope, selection)),
        WorkFilterField::Agent => match filter.agents.as_slice() {
            [] => None,
            [agent] => {
                let (key, id) = match agent {
                    ExecutionAgent::Builtin { id } => (format!("builtin:{id}"), id),
                    ExecutionAgent::Acp { id } => (format!("acp:{id}"), id),
                };
                Some(agent_choices(app, i18n).into_iter().find(|(value, _)| *value == key)
                    .map_or_else(|| id.clone(), |(_, label)| label))
            }
            agents => Some(count(i18n, "已选 %u 项", agents.len())),
        },
        WorkFilterField::Label => {
            let value = match filter.label_ids.as_slice() {
                [] => return None,
                [id] => app.task_board.task_labels.iter().find(|label| &label.id == id)
                    .map_or_else(|| id.clone(), |label| label.name.clone()),
                ids => count(i18n, "已选 %u 项", ids.len()),
            };
            Some(excluded_label(value, filter.labels_excluded))
        }
        WorkFilterField::Property(id) => {
            let property = app.task_board.task_properties.iter().find(|property| &property.id == id);
            let value = filter.properties.get(id);
            let unset = filter.unset_properties.contains(id);
            (value.is_some() || unset).then(|| super::properties::filter_value_label(i18n, property, value, unset))
        }
        WorkFilterField::Advanced => filter.advanced.as_ref()
            .map(|group| group.condition_count())
            .filter(|count| *count > 0)
            .map(|size| count(i18n, "%u 个条件", size)),
    }
}

fn count(i18n: I18n, template: &str, count: usize) -> String {
    i18n.format(template, &[count.into()]).expect("valid filter count")
}

fn value_text(value: String) -> Element<'static, Message> {
    container(SingleLineLabel::new(value, 13.0).shrink()).max_width(138).into()
}

fn chip(
    app: &App,
    i18n: I18n,
    field: WorkFilterField,
    icon: Icon,
    value: Element<'static, Message>,
) -> Element<'static, Message> {
    let label = if field == WorkFilterField::Date {
        app.task_board.work_filter.date.map_or_else(
            || field_label(app, i18n, &field),
            |date| i18n.__(date.field.label()).to_owned(),
        )
    } else {
        field_label(app, i18n, &field)
    };
    let hint = i18n.format("移除%s筛选", &[label.as_str().into()]).expect("valid filter action");
    let advanced = field == WorkFilterField::Advanced;
    let remove = button(icon_svg(Icon::X, 12.0).style(icon_btn_svg_style))
        .padding(4)
        .style(rounded_action_btn_style)
        .on_press(TaskBoardMessage::RemoveWorkFilter(field).into());
    let remove = tooltip(
        remove,
        container(text(hint).size(12)).padding([5, 8]).style(tooltip_dark_style),
        tooltip::Position::Top,
    )
    .gap(5);
    let summary = row![
        icon_svg(icon, 13.0).style(icon_btn_svg_style),
        container(SingleLineLabel::new(label, 12.0).shrink()).max_width(100),
        value,
    ].spacing(5).align_y(Alignment::Center);
    let summary: Element<'static, Message> = if advanced {
        button(summary).padding([2, 3])
            .style(|theme, status| super::style::navigation_button(theme, status, false))
            .on_press(TaskBoardMessage::AdvancedWorkFilter(AdvancedWorkFilterAction::Open).into())
            .into()
    } else {
        summary.into()
    };
    container(row![summary, remove].spacing(5).align_y(Alignment::Center))
    .padding([4, 6])
    .style(super::style::filter_chip_surface)
    .into()
}
