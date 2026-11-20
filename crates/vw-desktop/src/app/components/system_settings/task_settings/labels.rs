//! 项目任务标签目录及编辑表单。

use crate::app::components::system_settings_common::{
    primary_action_btn_style, rounded_action_btn_style, settings_close_button, settings_divider,
    settings_error_banner, settings_field_label, settings_group_card, settings_modal_header,
    settings_modal_layout, settings_muted_text_style, settings_page_heading, settings_text_input,
};
use crate::app::i18n::I18n;
use crate::app::message::task_board::{
    next_agent_scheduler_status_generation, resolve_task_board_scheduler_scope,
};
use crate::app::{App, Message};
use iced::widget::{button, column, row, text};
use iced::{Alignment, Element, Length, Task};
use vw_api_agent_scheduler::{AgentSchedulerTaskLabelDto, AgentSchedulerTaskLabelMutation};

const DEFAULT_COLOR: &str = "#4D7CFE";

#[derive(Default)]
pub(in crate::app) struct State {
    project_path: Option<String>,
    labels: Vec<AgentSchedulerTaskLabelDto>,
    draft: Option<Draft>,
    pending: Option<u64>,
    error: Option<String>,
}

struct Draft {
    original: Option<AgentSchedulerTaskLabelDto>,
    name: String,
    color: String,
}

#[derive(Debug, Clone)]
pub enum Event {
    Refresh,
    New,
    Edit(String),
    NameChanged(String),
    ColorChanged(String),
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
        let state = &mut app.task_label_settings;
        if state.pending != Some(request_id) || state.project_path.as_deref() != Some(project_path.as_str()) {
            return Task::none();
        }
        state.pending = None;
        match result {
            Ok(labels) => {
                state.labels = labels;
                if changed {
                    state.draft = None;
                }
                let catalog_generation = if changed {
                    next_agent_scheduler_status_generation()
                } else {
                    request_id
                };
                if app.navigation_tabs.project_path.as_deref() == Some(project_path.as_str())
                    && catalog_generation >= app.task_board.task_labels_generation
                {
                    app.task_board.task_labels.clone_from(&state.labels);
                    app.task_board.task_labels_generation = catalog_generation;
                }
                if changed {
                    return app.show_localized_toast(
                        crate::app::state::ToastKind::Success, "标签目录已保存", vec![],
                    );
                }
            }
            Err(error) => {
                state.error = Some(error.clone());
                if changed {
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
                color: DEFAULT_COLOR.to_owned(),
            });
            return Task::none();
        }
        Event::Edit(id) => {
            if let Some(label) = state.labels.iter().find(|label| label.id == id && !label.archived) {
                state.error = None;
                state.draft = Some(Draft {
                    original: Some(label.clone()),
                    name: label.name.clone(),
                    color: label.color.clone(),
                });
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
            Some(match draft.original.as_ref() {
                Some(original) => AgentSchedulerTaskLabelMutation::Update {
                    id: original.id.clone(),
                    name: (name != original.name).then_some(name),
                    color: (color != original.color).then_some(color),
                },
                None => AgentSchedulerTaskLabelMutation::Create { name, color },
            })
        }
        Event::Archive(id) => Some(AgentSchedulerTaskLabelMutation::Archive { id }),
        Event::Loaded { .. } => return Task::none(),
    };

    let request_id = next_agent_scheduler_status_generation();
    state.pending = Some(request_id);
    state.error = None;
    let changed = mutation.is_some();
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
        move |result| Message::TaskLabelSettings(Event::Loaded {
            project_path: result_project_path.clone(),
            request_id,
            changed,
            result,
        }),
    )
}

fn action<'a>(title: &'a str, event: Event, enabled: bool) -> button::Button<'a, Message> {
    button(text(title).size(13))
        .padding([7, 12])
        .style(rounded_action_btn_style)
        .on_press_maybe(enabled.then_some(Message::TaskLabelSettings(event)))
}

pub(super) fn view(app: &App) -> Element<'_, Message> {
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let title = i18n.__("标签");
    let Some(project_path) = super::project_path(app) else {
        return settings_page_heading(title, i18n.__("请先选择项目"));
    };
    let mut content = column![settings_page_heading(title, project_path)].spacing(12);
    let state = &app.task_label_settings;
    if state.project_path.as_deref() != Some(project_path) {
        return content.push(action(i18n.__("加载标签"), Event::Refresh, true)).into();
    }
    let enabled = state.pending.is_none();
    content = content.push(row![
        button(text(i18n.__("新建标签")).size(13))
            .padding([7, 12])
            .style(primary_action_btn_style)
            .on_press_maybe(enabled.then_some(Message::TaskLabelSettings(Event::New))),
        action(i18n.__("刷新"), Event::Refresh, enabled),
    ].spacing(8));
    if let Some(error) = state.error.as_ref() {
        content = content.push(settings_error_banner(error));
    }
    if state.labels.is_empty() {
        let empty = if enabled { "项目暂无标签" } else { "正在加载标签…" };
        return content.push(text(i18n.__(empty)).size(13).style(settings_muted_text_style)).into();
    }

    let mut list = column![].spacing(8);
    for (index, label) in state.labels.iter().enumerate() {
        if index > 0 {
            list = list.push(settings_divider());
        }
        let mut item = row![
            desktop_task_board::board::task_label_badge(label),
            iced::widget::Space::new().width(Length::Fill),
        ].spacing(8).align_y(Alignment::Center);
        if label.archived {
            item = item.push(text(i18n.__("已归档")).size(12).style(settings_muted_text_style));
        } else {
            item = item.push(action(i18n.__("编辑"), Event::Edit(label.id.clone()), enabled))
                .push(action(i18n.__("归档"), Event::Archive(label.id.clone()), enabled));
        }
        list = list.push(item.wrap());
    }
    content.push(settings_group_card(list)).into()
}

pub(in crate::app) fn overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let state = &app.task_label_settings;
    let Some(draft) = state.draft.as_ref().filter(|_| {
        super::project_path(app) == state.project_path.as_deref()
    }) else {
        return base;
    };
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let title = if draft.original.is_some() { "编辑标签" } else { "新建标签" };
    let close = Message::TaskLabelSettings(Event::Close);
    let header = settings_modal_header(text(i18n.__(title)).size(16), settings_close_button(close.clone()));
    let enabled = state.pending.is_none();
    let name = settings_text_input(i18n.__("标签名称"), &draft.name);
    let color = settings_text_input("#RRGGBB", &draft.color);
    let name = if enabled {
        name.on_input(|value| Message::TaskLabelSettings(Event::NameChanged(value)))
    } else {
        name
    };
    let color = if enabled {
        color.on_input(|value| Message::TaskLabelSettings(Event::ColorChanged(value)))
    } else {
        color
    };
    let mut body = column![
        settings_field_label(i18n.__("标签名称")),
        name,
        settings_field_label(i18n.__("颜色")),
        color,
    ].spacing(8);
    if let Some(error) = state.error.as_ref() {
        body = body.push(settings_error_banner(error));
    }
    let save = button(text(i18n.__("保存")).size(13))
        .padding([8, 14])
        .style(primary_action_btn_style)
        .on_press_maybe((enabled && !draft.name.trim().is_empty()).then_some(
            Message::TaskLabelSettings(Event::Save),
        ));
    let footer = row![action(i18n.__("取消"), Event::Close, enabled), save].spacing(8);
    let card = settings_modal_layout(header, body, Some(footer.into())).width(460).height(330);
    desktop_ui_controls::system_settings_common::panels::settings_modal_overlay_with_escape(base, close, card)
}
