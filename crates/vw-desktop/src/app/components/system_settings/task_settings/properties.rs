//! 当前项目的任务属性目录。

use crate::app::components::system_settings_common::*;
use crate::app::i18n::I18n;
use crate::app::message::task_board::{
    next_agent_scheduler_status_generation, resolve_task_board_scheduler_scope,
};
use crate::app::{App, Message};
use iced::widget::{button, column, row, text};
use iced::{Alignment, Element, Length, Task};
use vw_api_agent_scheduler::{
    AgentSchedulerTaskPropertyDto as Property, AgentSchedulerTaskPropertyKind as Kind,
    AgentSchedulerTaskPropertyMutation as Mutation, AgentSchedulerTaskPropertyOption as PropertyOption,
    AgentSchedulerTaskPropertyOptionChange as OptionChange,
};

#[derive(Default)]
pub(in crate::app) struct State {
    project_path: Option<String>,
    properties: Vec<Property>,
    draft: Option<Draft>,
    pending: Option<u64>,
    error: Option<String>,
}

struct Draft {
    original: Option<Property>,
    name: String,
    kind: Kind,
    options: Vec<OptionDraft>,
}

#[derive(Default)]
struct OptionDraft {
    original: Option<PropertyOption>,
    name: String,
    archive: bool,
}

#[derive(Debug, Clone)]
pub enum Event {
    Refresh,
    New,
    Edit(String),
    NameChanged(String),
    KindChanged(Kind),
    OptionChanged(usize, String),
    AddOption,
    RemoveOption(usize),
    ToggleOptionArchive(usize),
    Close,
    Save,
    Archive(String),
    Loaded {
        project_path: String,
        request_id: u64,
        changed: bool,
        result: Result<Vec<Property>, String>,
    },
}

pub(in crate::app) fn update(app: &mut App, event: Event) -> Task<Message> {
    if let Event::Loaded { project_path, request_id, changed, result } = event {
        let selected = super::project_path(app) == Some(project_path.as_str());
        let state = &mut app.task_property_settings;
        if state.pending != Some(request_id) || state.project_path.as_deref() != Some(project_path.as_str()) {
            return Task::none();
        }
        state.pending = None;
        match result {
            Ok(properties) => {
                state.properties = properties;
                if changed {
                    state.draft = None;
                }
                let generation = if changed {
                    next_agent_scheduler_status_generation()
                } else {
                    request_id
                };
                if app.navigation_tabs.project_path.as_deref() == Some(project_path.as_str())
                    && generation >= app.task_board.task_properties_generation
                {
                    app.task_board.task_properties.clone_from(&state.properties);
                    app.task_board.task_properties_generation = generation;
                }
                if changed && selected {
                    return app.show_localized_toast(
                        crate::app::state::ToastKind::Success, "属性目录已保存", vec![],
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
    let state = &mut app.task_property_settings;
    if state.project_path.as_deref() != Some(project_path.as_str()) {
        if !matches!(event, Event::Refresh) {
            return Task::none();
        }
        *state = State { project_path: Some(project_path.clone()), ..State::default() };
    }
    if state.pending.is_some() {
        return Task::none();
    }

    let mutation = match event {
        Event::Refresh => None,
        Event::Archive(id) => Some(Mutation::Archive { id }),
        Event::Save => {
            let Some(draft) = state.draft.as_ref() else {
                return Task::none();
            };
            let new_options = draft.options.iter()
                .filter(|option| option.original.is_none() && draft.kind == Kind::SingleSelect)
                .map(|option| option.name.clone())
                .collect();
            Some(if let Some(original) = draft.original.as_ref() {
                let option_changes = draft.options.iter().filter_map(|option| {
                    let original = option.original.as_ref()?;
                    let name = (option.name != original.name).then(|| option.name.clone());
                    (name.is_some() || option.archive).then(|| (
                        original.id.clone(),
                        OptionChange { name, archive: option.archive },
                    ))
                }).collect();
                Mutation::Update {
                    id: original.id.clone(),
                    name: (draft.name != original.name).then(|| draft.name.clone()),
                    new_options,
                    option_changes,
                }
            } else {
                Mutation::Create { name: draft.name.clone(), kind: draft.kind, options: new_options }
            })
        }
        Event::New => {
            state.error = None;
            state.draft = Some(Draft {
                original: None,
                name: String::new(),
                kind: Kind::Text,
                options: vec![OptionDraft::default()],
            });
            return Task::none();
        }
        Event::Edit(id) => {
            if let Some(property) = state.properties.iter().find(|property| property.id == id && !property.archived) {
                state.error = None;
                state.draft = Some(Draft {
                    original: Some(property.clone()),
                    name: property.name.clone(),
                    kind: property.kind,
                    options: property.options.iter().map(|option| OptionDraft {
                        original: Some(option.clone()),
                        name: option.name.clone(),
                        archive: false,
                    }).collect(),
                });
            }
            return Task::none();
        }
        Event::Close => {
            state.draft = None;
            state.error = None;
            return Task::none();
        }
        event => {
            if let Some(draft) = state.draft.as_mut() {
                match event {
                    Event::NameChanged(value) => draft.name = value,
                    Event::KindChanged(kind) if draft.original.is_none() => draft.kind = kind,
                    Event::AddOption => draft.options.push(OptionDraft::default()),
                    Event::RemoveOption(index) => {
                        if draft.options.get(index).is_some_and(|option| option.original.is_none())
                            && (draft.original.is_some() || draft.options.len() > 1)
                        {
                            draft.options.remove(index);
                        }
                    }
                    Event::ToggleOptionArchive(index) => {
                        if let Some(option) = draft.options.get_mut(index)
                            .filter(|option| option.original.as_ref().is_some_and(|original| !original.archived))
                        {
                            option.archive = !option.archive;
                        }
                    }
                    Event::OptionChanged(index, value) => {
                        if let Some(option) = draft.options.get_mut(index) {
                            option.name = value;
                        }
                    }
                    _ => {}
                }
            }
            return Task::none();
        }
    };

    let request_id = next_agent_scheduler_status_generation();
    state.pending = Some(request_id);
    state.error = None;
    let changed = mutation.is_some();
    let result_project = project_path.clone();
    Task::perform(
        async move {
            let scope = resolve_task_board_scheduler_scope(&project_path).await?;
            let client = crate::app::config::gateway_client()?;
            if let Some(mutation) = mutation {
                client.agent_scheduler_mutate_task_properties(&scope, &mutation).await
            } else {
                client.agent_scheduler_task_properties(&scope).await
            }
        },
        move |result| Message::TaskPropertySettings(Event::Loaded {
            project_path: result_project.clone(), request_id, changed, result,
        }),
    )
}

fn kind_label(i18n: I18n, kind: Kind) -> &'static str {
    i18n.__(match kind {
        Kind::SingleSelect => "单选",
        Kind::Date => "日期",
        Kind::Text => "文本",
    })
}

fn action<'a>(label: &'a str, event: Event, enabled: bool) -> button::Button<'a, Message> {
    button(text(label).size(13))
        .padding([7, 12])
        .style(rounded_action_btn_style)
        .on_press_maybe(enabled.then_some(Message::TaskPropertySettings(event)))
}

pub(super) fn view(app: &App) -> Element<'_, Message> {
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let title = i18n.__("属性");
    let Some(project_path) = super::project_path(app) else {
        return settings_page_heading(title, i18n.__("请先选择项目"));
    };
    let state = &app.task_property_settings;
    let examples = settings_hover_help_label(
        i18n,
        &app.appearance.app_theme,
        i18n.__("属性示例"),
        settings_field_label(i18n.__("属性示例")),
        i18n.__("用日期记录验收日，用单选标记需求来源（如客户反馈），用文本填写验收说明。归档后保留任务中的历史值。"),
    );
    let mut content = column![settings_page_heading(title, project_path), examples].spacing(12);
    if state.project_path.as_deref() != Some(project_path) {
        return content.push(action(i18n.__("加载属性"), Event::Refresh, true)).into();
    }
    let enabled = state.pending.is_none();
    content = content.push(row![
        button(text(i18n.__("新建属性")).size(13))
            .padding([7, 12])
            .style(primary_action_btn_style)
            .on_press_maybe(enabled.then_some(Message::TaskPropertySettings(Event::New))),
        action(i18n.__("刷新"), Event::Refresh, enabled),
    ].spacing(8));
    if let Some(error) = state.error.as_ref() {
        content = content.push(settings_error_banner(error));
    }
    if state.properties.is_empty() {
        let status = if enabled { "项目暂无属性" } else { "正在加载属性…" };
        return content.push(text(i18n.__(status)).size(13).style(settings_muted_text_style)).into();
    }

    let mut list = column![].spacing(8);
    for (index, property) in state.properties.iter().enumerate() {
        if index > 0 {
            list = list.push(settings_divider());
        }
        let mut details = column![
            settings_field_label(&property.name),
            text(kind_label(i18n, property.kind)).size(12).style(settings_muted_text_style),
        ].spacing(4).width(Length::Fill);
        if !property.options.is_empty() {
            let options = property.options.iter().filter(|option| !option.archived)
                .map(|option| option.name.as_str()).collect::<Vec<_>>();
            details = details.push(text(options.join(" · ")).size(12).style(settings_muted_text_style));
        }
        let item = row![details].spacing(8).align_y(Alignment::Center);
        let item = if property.archived {
            item.push(text(i18n.__("已归档")).size(12).style(settings_muted_text_style))
        } else {
            item.push(action(i18n.__("编辑"), Event::Edit(property.id.clone()), enabled))
                .push(action(i18n.__("归档"), Event::Archive(property.id.clone()), enabled))
        };
        list = list.push(item.wrap());
    }
    content.push(settings_group_card(list)).into()
}

pub(in crate::app) fn overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let state = &app.task_property_settings;
    let Some(draft) = state.draft.as_ref().filter(|_| {
        super::project_path(app) == state.project_path.as_deref()
    }) else {
        return base;
    };
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let enabled = state.pending.is_none();
    let title = if draft.original.is_some() { "编辑属性" } else { "新建属性" };
    let close = Message::TaskPropertySettings(Event::Close);
    let header = settings_modal_header(text(i18n.__(title)).size(16), settings_close_button(close.clone()));
    let name = settings_text_input(i18n.__("属性名称"), &draft.name);
    let name = if enabled {
        name.on_input(|value| Message::TaskPropertySettings(Event::NameChanged(value)))
    } else {
        name
    };
    let mut body = column![settings_field_label(i18n.__("属性名称")), name].spacing(8);
    let choosing_options = draft.kind == Kind::SingleSelect;
    if draft.original.is_none() {
        let choices = [Kind::SingleSelect, Kind::Date, Kind::Text].into_iter().map(|kind| {
            button(text(kind_label(i18n, kind)).size(13))
                .padding([7, 12])
                .style(if draft.kind == kind { primary_action_btn_style } else { rounded_action_btn_style })
                .on_press_maybe(enabled.then_some(Message::TaskPropertySettings(Event::KindChanged(kind))))
                .into()
        });
        body = body.push(settings_field_label(i18n.__("类型"))).push(row(choices).spacing(8).wrap());
    }
    if choosing_options {
        body = body.push(settings_field_label(i18n.__("单选选项")));
        for (index, option) in draft.options.iter().enumerate() {
            let input = settings_text_input(i18n.__("选项名称"), &option.name);
            let archived = option.original.as_ref().is_some_and(|original| original.archived);
            let input = if enabled && !archived && !option.archive {
                input.on_input(move |value| Message::TaskPropertySettings(Event::OptionChanged(index, value)))
            } else {
                input
            };
            let action_control: Element<'_, Message> = if archived {
                text(i18n.__("已归档")).size(12).style(settings_muted_text_style).into()
            } else if option.original.is_some() {
                let label = if option.archive { "撤销归档" } else { "归档" };
                action(i18n.__(label), Event::ToggleOptionArchive(index), enabled).into()
            } else {
                action(
                    i18n.__("移除"),
                    Event::RemoveOption(index),
                    enabled && (draft.original.is_some() || draft.options.len() > 1),
                ).into()
            };
            body = body.push(row![input, action_control].spacing(8).align_y(Alignment::Center));
        }
        body = body.push(action(i18n.__("添加选项"), Event::AddOption, enabled));
    }
    if let Some(error) = state.error.as_ref() {
        body = body.push(settings_error_banner(error));
    }
    let valid = !draft.name.trim().is_empty()
        && (!choosing_options || draft.options.iter().all(|option| !option.name.trim().is_empty()));
    let save = button(text(i18n.__("保存")).size(13))
        .padding([8, 14])
        .style(primary_action_btn_style)
        .on_press_maybe((enabled && valid).then_some(Message::TaskPropertySettings(Event::Save)));
    let footer = row![action(i18n.__("取消"), Event::Close, enabled), save].spacing(8);
    let card = settings_modal_layout(header, body, Some(footer.into())).width(460).height(400);
    desktop_ui_controls::system_settings_common::panels::settings_modal_overlay_with_escape(base, close, card)
}
