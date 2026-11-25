//! 项目任务标签目录及编辑表单。

use super::catalog;
use crate::app::components::system_settings_common::*;
use crate::app::i18n::I18n;
use crate::app::message::task_board::{
    next_agent_scheduler_status_generation, resolve_task_board_scheduler_scope,
};
use crate::app::{App, Message};
use desktop_assets::assets::Icon;
use desktop_ui_controls::system_settings_common::panels::settings_responsive_row;
use iced::widget::{column, container, operation, row, text, text_editor};
use iced::{Alignment, Element, Length, Task};
use vw_api_agent_scheduler::{AgentSchedulerTaskLabelDto, AgentSchedulerTaskLabelMutation};

const NAME_INPUT: &str = "task-label-setting-name";

#[derive(Default)]
pub(in crate::app) struct State {
    project_path: Option<String>,
    labels: Vec<AgentSchedulerTaskLabelDto>,
    query: String,
    show_archived: bool,
    draft: Option<Draft>,
    pending: Option<u64>,
    saving: bool,
    error: Option<String>,
}

impl State {
    pub(super) fn saving(&self) -> bool {
        self.saving
    }
}

struct Draft {
    original: Option<AgentSchedulerTaskLabelDto>,
    name: String,
    color: String,
    description: text_editor::Content,
    picker_open: bool,
}

#[derive(Debug, Clone)]
pub enum Event {
    Refresh,
    New,
    Edit(String),
    NameChanged(String),
    ColorChanged(String),
    DescriptionChanged(text_editor::Action),
    ColorPicker(bool),
    Search(String),
    ShowArchived(bool),
    Close,
    Save,
    Archive(String),
    Loaded {
        project_path: String,
        request_id: u64,
        changed: bool,
        result: Result<Vec<AgentSchedulerTaskLabelDto>, String>,
    },
}

pub(in crate::app) fn update(app: &mut App, event: Event) -> Task<Message> {
    if let Event::Loaded { project_path, request_id, changed, result } = event {
        let selected = super::project_path(app) == Some(project_path.as_str());
        let state = &mut app.task_label_settings;
        if state.pending != Some(request_id)
            || state.project_path.as_deref() != Some(project_path.as_str())
        {
            return Task::none();
        }
        state.pending = None;
        state.saving = false;
        match result {
            Ok(labels) => {
                state.labels = labels;
                if changed {
                    state.draft = None;
                }
                let catalog_generation =
                    if changed { next_agent_scheduler_status_generation() } else { request_id };
                if app.navigation_tabs.project_path.as_deref() == Some(project_path.as_str())
                    && catalog_generation >= app.task_board.task_labels_generation
                {
                    app.task_board.task_labels.clone_from(&state.labels);
                    app.task_board.task_labels_generation = catalog_generation;
                }
                if changed && selected {
                    return app.show_localized_toast(
                        crate::app::state::ToastKind::Success,
                        "标签目录已保存",
                        vec![],
                    );
                }
            }
            Err(error) => {
                state.error = Some(error.clone());
                if changed && selected {
                    return app.show_error_toast(error);
                }
            }
        }
        return Task::none();
    }

    let Some(project_path) = super::project_path(app).map(str::to_owned) else {
        return Task::none();
    };
    let state = &mut app.task_label_settings;
    if state.project_path.as_ref() != Some(&project_path) {
        if !matches!(&event, Event::Refresh) {
            return Task::none();
        }
        *state = State { project_path: Some(project_path.clone()), ..State::default() };
    }
    match event {
        Event::Search(value) => {
            state.query = value;
            return Task::none();
        }
        Event::ShowArchived(show) => {
            state.show_archived = show;
            return Task::none();
        }
        _ => {}
    }
    if state.pending.is_some() {
        return Task::none();
    }

    let mutation = match event {
        Event::Refresh => None,
        Event::New => {
            state.error = None;
            state.draft = Some(Draft {
                original: None,
                name: String::new(),
                color: catalog::DEFAULT_COLOR.to_owned(),
                description: text_editor::Content::new(),
                picker_open: false,
            });
            return operation::focus(NAME_INPUT);
        }
        Event::Edit(id) => {
            if let Some(label) = state.labels.iter().find(|label| label.id == id && !label.archived)
            {
                state.error = None;
                state.draft = Some(Draft {
                    original: Some(label.clone()),
                    name: label.name.clone(),
                    color: label.color.clone(),
                    description: text_editor::Content::with_text(&label.description),
                    picker_open: false,
                });
                return operation::focus(NAME_INPUT);
            }
            return Task::none();
        }
        Event::NameChanged(value) => {
            if let Some(draft) = state.draft.as_mut() {
                draft.name = value;
            }
            return Task::none();
        }
        Event::ColorChanged(value) => {
            if let Some(draft) = state.draft.as_mut() {
                draft.color = value;
            }
            return Task::none();
        }
        Event::DescriptionChanged(action) => {
            if let Some(draft) = state.draft.as_mut() {
                draft.description.perform(action);
            }
            return Task::none();
        }
        Event::ColorPicker(open) => {
            if let Some(draft) = state.draft.as_mut() {
                draft.picker_open = open;
            }
            return Task::none();
        }
        Event::Close => {
            state.draft = None;
            state.error = None;
            return Task::none();
        }
        Event::Save => {
            let Some(draft) = state.draft.as_ref() else {
                return Task::none();
            };
            let name = draft.name.trim().to_owned();
            let color = draft.color.trim().to_ascii_uppercase();
            if name.is_empty() || catalog::palette_color(&color).is_none() {
                return Task::none();
            }
            let description = draft.description.text().trim().to_owned();
            Some(match draft.original.as_ref() {
                Some(original) => AgentSchedulerTaskLabelMutation::Update {
                    id: original.id.clone(),
                    name: (name != original.name).then_some(name),
                    color: (color != original.color).then_some(color),
                    description: (description != original.description).then_some(description),
                },
                None => AgentSchedulerTaskLabelMutation::Create { name, color, description },
            })
        }
        Event::Archive(id) => Some(AgentSchedulerTaskLabelMutation::Archive { id }),
        Event::Loaded { .. } | Event::Search(_) | Event::ShowArchived(_) => return Task::none(),
    };

    let request_id = next_agent_scheduler_status_generation();
    state.pending = Some(request_id);
    state.error = None;
    let changed = mutation.is_some();
    state.saving = changed;
    let result_project_path = project_path.clone();
    Task::perform(
        async move {
            let scope = resolve_task_board_scheduler_scope(&project_path).await?;
            let client = crate::app::config::gateway_client()?;
            if let Some(mutation) = mutation {
                client.agent_scheduler_mutate_task_labels(&scope, &mutation).await
            } else {
                client.agent_scheduler_task_labels(&scope).await
            }
        },
        move |result| {
            Message::TaskLabelSettings(Event::Loaded {
                project_path: result_project_path.clone(),
                request_id,
                changed,
                result,
            })
        },
    )
}

fn message(event: Event) -> Message {
    Message::TaskLabelSettings(event)
}

pub(super) fn view(app: &App) -> Element<'_, Message> {
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let title = settings_page_heading(i18n.__("标签"), i18n.__("为项目任务维护可复用的标签。"));
    let mut content = column![title].spacing(18);
    let Some(project) = super::project_path(app) else {
        return content.into();
    };
    let state = &app.task_label_settings;
    if state.project_path.as_deref() != Some(project) {
        return content
            .push(catalog::action(i18n.__("加载标签"), Some(message(Event::Refresh))))
            .into();
    }
    let enabled = state.pending.is_none();
    let query = state.query.trim().to_lowercase();
    let search = settings_text_input(i18n.__("按名称或描述搜索"), &state.query)
        .on_input(|value| message(Event::Search(value)));
    let create = catalog::primary(
        i18n.__("新建标签"),
        Some(Icon::Plus),
        enabled.then_some(message(Event::New)),
    );
    content = content.push(catalog::toolbar(search.into(), create));
    content = content.push(
        settings_toggle(state.show_archived)
            .label(i18n.__("显示已归档"))
            .text_size(13)
            .on_toggle(|show| message(Event::ShowArchived(show))),
    );
    if let Some(error) = state.error.as_ref() {
        content = content.push(
            row![
                container(settings_error_banner(error)).width(Length::Fill),
                catalog::action(i18n.__("刷新"), enabled.then_some(message(Event::Refresh))),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        );
    }
    let mut list = column![catalog::record(
        [(catalog::cell(i18n.__("名称")).into(), 2), (catalog::cell(i18n.__("描述")).into(), 3),],
        None
    )];
    let mut has_results = false;
    for label in &state.labels {
        if (label.archived && !state.show_archived)
            || !(catalog::matches_search(&label.name, &query)
                || catalog::matches_search(&label.description, &query))
        {
            continue;
        }
        has_results = true;
        let mut name = row![].spacing(9).align_y(Alignment::Center);
        if let Some(color) = catalog::palette_color(&label.color) {
            name = name.push(catalog::color_dot(color, 10.0));
        }
        name = name.push(text(&label.name).size(14));
        let name: Element<'_, Message> = if label.archived {
            column![name, catalog::cell(i18n.__("已归档"))].spacing(3).into()
        } else {
            name.into()
        };
        let menu = (!label.archived).then(|| {
            catalog::menu(
                vec![
                    (i18n.__("编辑"), message(Event::Edit(label.id.clone()))),
                    (i18n.__("归档"), message(Event::Archive(label.id.clone()))),
                ],
                enabled,
            )
        });
        let description = if label.description.is_empty() { "—" } else { &label.description };
        list = list.push(settings_divider()).push(catalog::record(
            [(name, 2), (catalog::cell(catalog::preview(description)).into(), 3)],
            menu,
        ));
    }
    if !has_results {
        let status = if !enabled {
            "正在加载标签…"
        } else if state.labels.is_empty() && query.is_empty() {
            "项目暂无标签"
        } else {
            "没有匹配的结果"
        };
        list = list
            .push(settings_divider())
            .push(container(catalog::cell(i18n.__(status))).padding([18, 0]));
    }
    content.push(settings_group_card(list)).into()
}

pub(in crate::app) fn overlay<'a>(
    app: &'a App,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    let state = &app.task_label_settings;
    let Some(draft) =
        state.draft.as_ref().filter(|_| super::project_path(app) == state.project_path.as_deref())
    else {
        return base;
    };
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let title = if draft.original.is_some() { "编辑标签" } else { "新建标签" };
    let enabled = state.pending.is_none();
    let mut body = column![
        settings_responsive_row(i18n.__("名称"), "", move || {
            let input = settings_text_input(i18n.__("标签名称"), &draft.name).id(NAME_INPUT);
            if enabled { input.on_input(|value| message(Event::NameChanged(value))) } else { input }
        }),
        settings_divider(),
        settings_responsive_row(i18n.__("描述"), "", move || {
            let input = settings_text_editor(&draft.description)
                .height(80)
                .placeholder(i18n.__("说明何时使用这个标签"));
            if enabled {
                input.on_action(|action| message(Event::DescriptionChanged(action)))
            } else {
                input
            }
        }),
        settings_divider(),
        settings_responsive_row(i18n.__("颜色"), "", move || {
            catalog::color_control(
                i18n,
                &draft.color,
                true,
                enabled && draft.picker_open,
                enabled.then_some(message(Event::ColorPicker(!draft.picker_open))),
                message(Event::ColorPicker(false)),
                |value| message(Event::ColorChanged(value)),
            )
        }),
    ]
    .spacing(8);
    if let Some(error) = state.error.as_ref() {
        body = body.push(settings_error_banner(error));
    }
    let valid =
        enabled && !draft.name.trim().is_empty() && catalog::palette_color(&draft.color).is_some();
    catalog::modal(
        i18n,
        base,
        i18n.__(title),
        body,
        390.0,
        message(Event::Close),
        valid.then_some(message(Event::Save)),
    )
}
