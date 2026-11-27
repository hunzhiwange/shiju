use crate::app::components::system_settings_common::{icon_svg, settings_muted_text_style};
use crate::app::i18n::I18n;
use crate::app::message::TaskBoardMessage;
use crate::app::{App, Message};
use desktop_assets::assets::Icon;
use desktop_task_board::work_views::{
    WORK_DUE_DATES, WORK_RELATIONS, WorkScopeField, WorkScopeSelection,
};
use desktop_task_model::{TASK_AGENT_MAIN, Task, normalize_task_acp_agent_input};
use desktop_ui_controls::system_settings_common::icons::icon_btn_svg_style;
use iced::widget::{column, container, text};
use iced::{Color, Element, Length, Padding};
use std::collections::{BTreeMap, BTreeSet};
use vw_api_agent_scheduler::AgentSchedulerProjectProgress as Progress;

use crate::app::components::system_settings::task_settings::catalog::color_dot;

pub(in crate::app) fn configured_work_agent(app: &App, task: &Task) -> bool {
    if let Some(acp) = task.acp_agent.as_deref().and_then(normalize_task_acp_agent_input) {
        return app.acp_settings.catalog.contains_key(&acp);
    }
    let agent = task.agent.as_deref().unwrap_or(TASK_AGENT_MAIN);
    agent == TASK_AGENT_MAIN || app.agents_settings.entries.iter().any(|entry| entry.key == agent)
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
enum Group {
    Ungrouped,
    Members,
    Agents,
    Teams,
}

impl Group {
    fn title(self) -> Option<&'static str> {
        match self {
            Self::Ungrouped => None,
            Self::Members => Some("成员"),
            Self::Agents => Some("智能体"),
            Self::Teams => Some("小队"),
        }
    }
}

pub(super) fn field_icon(field: WorkScopeField) -> Icon {
    match field {
        WorkScopeField::Assignee | WorkScopeField::Relation => Icon::Person,
        WorkScopeField::Creator => Icon::Pencil,
        WorkScopeField::Project => Icon::FolderClosed,
        WorkScopeField::ProjectStatus => Icon::Circle,
        WorkScopeField::DueDate => Icon::Calendar,
    }
}

pub(super) fn summary(
    app: &App,
    i18n: I18n,
    field: WorkScopeField,
    selection: &WorkScopeSelection,
) -> String {
    let value = if let Some(id) = selection.values.first().filter(|_| selection.values.len() == 1) {
        let is_team = field == WorkScopeField::Assignee
            && app
                .task_board
                .task_board_tasks
                .iter()
                .any(|task| task.assigned_team.as_deref() == Some(id));
        let (label, _, _) = presentation(app, i18n, field, id, is_team);
        label
    } else {
        i18n.format("已选 %u 项", &[selection.values.len().into()]).expect("valid selected count")
    };
    if selection.excluded { format!("{} {value}", i18n.__("排除")) } else { value }
}

pub(super) fn value_matches(app: &App, i18n: I18n, field: WorkScopeField, query: &str) -> bool {
    let has = |id: &str| {
        id.to_lowercase().contains(query)
            || presentation(app, i18n, field, id, false).0.to_lowercase().contains(query)
    };
    let board = &app.task_board;
    let live_tasks = || board.task_board_tasks.iter().filter(|task| !task.archived && !task.deleted);
    match field {
        WorkScopeField::Assignee => has("") || live_tasks().any(|task| has(task.assignee.trim())),
        WorkScopeField::Creator => live_tasks().any(|task| task.created_by.as_deref().is_some_and(has)),
        WorkScopeField::Relation => WORK_RELATIONS.iter().any(|(id, _)| {
            (board.work_viewer_id.is_some() || *id == "my_agent_running") && has(id)
        }),
        WorkScopeField::Project => board.work_snapshot_project_id.as_deref().is_some_and(has),
        WorkScopeField::ProjectStatus => Progress::ALL.iter().any(|progress| has(progress.key())),
        WorkScopeField::DueDate => WORK_DUE_DATES.iter().any(|(id, _)| has(id)),
    }
}

pub(super) fn choices(
    app: &App,
    i18n: I18n,
    field: WorkScopeField,
    query: &str,
) -> Element<'static, Message> {
    let board = &app.task_board;
    let selected = board.work_scope_selections.get(&field);
    let preset_row = |id: &str, label: &str, marker| {
        super::work_filter_picker::marked_choice(
            i18n.__(label),
            marker,
            selected.is_some_and(|selection| selection.values.contains(id)),
            TaskBoardMessage::ToggleWorkScopeValue(field, id.to_owned()),
        )
    };
    let live_tasks =
        || board.task_board_tasks.iter().filter(|task| !task.archived && !task.deleted);
    let teams: BTreeSet<_> = if field == WorkScopeField::Assignee {
        live_tasks().filter_map(|task| task.assigned_team.as_deref()).collect()
    } else {
        BTreeSet::new()
    };
    let mut options = BTreeMap::new();
    let mut add = |id: &str| {
        let (label, icon, group) = presentation(app, i18n, field, id, teams.contains(id));
        if label.to_lowercase().contains(query) || id.to_lowercase().contains(query) {
            options.entry(group).or_insert_with(BTreeMap::new).insert((label, id.to_owned()), icon);
        }
    };
    match field {
        WorkScopeField::ProjectStatus => {
            return column(Progress::ALL.map(|progress| {
                let (label, color) = progress_presentation(progress);
                preset_row(progress.key(), label, color_dot(color, 8.0))
            }))
            .spacing(4)
            .push(
                text(i18n.__("按当前项目的任务进度计算")).size(11).style(settings_muted_text_style),
            )
            .into();
        }
        WorkScopeField::DueDate => {
            return column(WORK_DUE_DATES.map(|(id, label)| {
                preset_row(
                    id,
                    label,
                    icon_svg(Icon::Calendar, 14.0).style(icon_btn_svg_style).into(),
                )
            }))
            .spacing(4)
            .into();
        }
        WorkScopeField::Assignee | WorkScopeField::Creator => {
            if field == WorkScopeField::Assignee {
                add("");
            }
            for task in live_tasks() {
                if field == WorkScopeField::Assignee {
                    add(task.assignee.trim());
                } else if let Some(creator) = task.created_by.as_deref() {
                    add(creator);
                }
            }
        }
        WorkScopeField::Relation => {
            for (id, _) in WORK_RELATIONS {
                if board.work_viewer_id.is_some() || id == "my_agent_running" {
                    add(id);
                }
            }
        }
        WorkScopeField::Project => {
            if let Some(project) = board.work_snapshot_project_id.as_deref() {
                add(project);
            } else if app.navigation_tabs.project_path.is_none() {
                add("");
            }
        }
    }
    if let Some(selection) = selected {
        for id in &selection.values {
            add(id);
        }
    }
    if options.is_empty() {
        return container(
            text(i18n.__("没有匹配的筛选项")).size(13).style(settings_muted_text_style),
        )
        .padding([7, 4])
        .into();
    }
    let mut list = column![].spacing(4).width(Length::Fill);
    for (group, values) in options {
        if let Some(title) = group.title() {
            list = list.push(
                container(text(i18n.__(title)).size(12).style(settings_muted_text_style))
                    .padding(Padding::from([5, 4]).bottom(1)),
            );
        }
        for ((label, id), icon) in values {
            let checked = selected.is_some_and(|selection| selection.values.contains(&id));
            list = list.push(super::work_filter_picker::marked_choice(
                &label,
                icon_svg(icon, 14.0).style(icon_btn_svg_style).into(),
                checked,
                TaskBoardMessage::ToggleWorkScopeValue(field, id),
            ));
        }
    }
    list.into()
}

fn progress_presentation(progress: Progress) -> (&'static str, Color) {
    match progress {
        Progress::Planned => ("计划中", Color::from_rgb8(115, 123, 139)),
        Progress::InProgress => ("进行中", Color::from_rgb8(205, 145, 22)),
        Progress::Paused => ("已暂停", Color::from_rgb8(129, 117, 147)),
        Progress::Completed => ("已完成", Color::from_rgb8(28, 116, 193)),
        Progress::Cancelled => ("已取消", Color::from_rgb8(211, 64, 76)),
    }
}

fn presentation(
    app: &App,
    i18n: I18n,
    field: WorkScopeField,
    id: &str,
    is_team: bool,
) -> (String, Icon, Group) {
    use Group::*;
    match field {
        WorkScopeField::Assignee if id.is_empty() => {
            (i18n.__("无负责人").to_owned(), Icon::Person, Ungrouped)
        }
        WorkScopeField::Assignee | WorkScopeField::Creator => {
            if is_team {
                return (id.to_owned(), Icon::Box, Teams);
            }
            if app.task_board.work_viewer_id.as_deref() == Some(id)
                || (field == WorkScopeField::Creator
                    && !id.starts_with("builtin:")
                    && !id.starts_with("acp:"))
            {
                return (super::detail::display_author(i18n, id).to_owned(), Icon::Person, Members);
            }
            let agent = id.strip_prefix("builtin:").unwrap_or(id);
            if agent == TASK_AGENT_MAIN {
                return (i18n.__("主智能体").to_owned(), Icon::Robot, Agents);
            }
            if let Some(entry) = app.agents_settings.entries.iter().find(|entry| {
                entry.key == agent || (field == WorkScopeField::Assignee && entry.label == id)
            }) {
                return (entry.label.clone(), Icon::Robot, Agents);
            }
            let acp = id.strip_prefix("acp:").unwrap_or(id);
            if app.acp_settings.catalog.contains_key(acp) {
                return (
                    desktop_task_board::draft::task_acp_agent_label(Some(acp)),
                    Icon::Robot,
                    Agents,
                );
            }
            (super::detail::display_author(i18n, id).to_owned(), Icon::Person, Members)
        }
        WorkScopeField::Relation => {
            let label = WORK_RELATIONS
                .iter()
                .find_map(|(key, label)| (*key == id).then_some(*label))
                .map_or_else(|| id.to_owned(), |label| i18n.__(label).to_owned());
            let icon = match id {
                "created_by_me" => Icon::Pencil,
                "my_agent_running" => Icon::Robot,
                "subscribed" => Icon::Eye,
                _ => Icon::Person,
            };
            (label, icon, Ungrouped)
        }
        WorkScopeField::Project => {
            let label = if id.is_empty() {
                i18n.__("无项目").to_owned()
            } else if app.task_board.work_snapshot_project_id.as_deref() == Some(id) {
                desktop_workspace::basic::display_name_for_path(
                    &app.workspace_state.recent_projects_meta,
                    app.navigation_tabs.project_path.as_deref().unwrap_or(id),
                )
            } else {
                id.to_owned()
            };
            (label, Icon::FolderClosed, Ungrouped)
        }
        WorkScopeField::ProjectStatus => {
            let label = Progress::ALL
                .into_iter()
                .find(|progress| progress.key() == id)
                .map_or(id, |progress| i18n.__(progress_presentation(progress).0));
            (label.to_owned(), Icon::Circle, Ungrouped)
        }
        WorkScopeField::DueDate => {
            let label = WORK_DUE_DATES
                .iter()
                .find(|(key, _)| *key == id)
                .map_or(id, |(_, label)| i18n.__(label));
            (label.to_owned(), Icon::Calendar, Ungrouped)
        }
    }
}
