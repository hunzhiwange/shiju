use super::catalog as ui;
use crate::app::components::system_settings_common::{
    settings_divider, settings_error_banner, settings_group_card, settings_help_button_with_locale,
    settings_page_heading, settings_text_editor, settings_text_input, with_settings_help_modal,
};
use crate::app::i18n::I18n;
use crate::app::message::task_quick_actions::{self as domain, Origin};
use crate::app::message::{SettingsMessage, ViewMessage, settings::AgentConfigTabMessage};
use crate::app::state::AgentConfigTab;
use crate::app::{App, Message};
use desktop_assets::assets::Icon;
use desktop_settings_shell::SystemTab;
use desktop_ui_controls::system_settings_common::panels::settings_responsive_row;
use iced::widget::{column, container, operation, row, text, text_editor};
use iced::{Alignment, Element, Length, Task};
use task_file_store::task_quick_actions::{self as catalog, TaskQuickAction};

const TITLE: &str = "任务快捷操作";
const NAME_INPUT: &str = "task-quick-action-name";
const HELP: &str = "模板保存在当前项目。在任务详情选择「总结进展」等操作，预览后会打开可编辑的会话草稿。只有你主动发送，才会发出请求。聊天快捷消息在「智能体」设置中单独管理，其中部分操作可以直接发送。";
const EXAMPLES: [(&str, &str); 2] = [
    ("总结进展", "请总结这项任务的当前进展、剩余工作和已知阻碍。"),
    ("检查验收项", "请逐项检查这项任务的验收要求，列出已满足的项目和仍需验证的内容。"),
];

#[derive(Clone, Debug)]
pub enum Event {
    Load,
    New,
    Example(usize),
    Edit(u64),
    Archive(u64),
    Name(String),
    Search(String),
    Prompt(text_editor::Action),
    Save,
    Close,
    Help(bool),
    Finished { request: u64, mutation: bool, result: Result<Vec<TaskQuickAction>, &'static str> },
}

#[derive(Default)]
pub(crate) struct State {
    origin: Option<Origin>,
    request: u64,
    busy: bool,
    saving: bool,
    actions: Vec<TaskQuickAction>,
    query: String,
    editor: Option<Editor>,
    error: Option<&'static str>,
    help: bool,
}

impl State {
    pub(super) fn saving(&self) -> bool {
        self.saving
    }
}

struct Editor {
    previous: Option<TaskQuickAction>,
    name: String,
    prompt: text_editor::Content,
}

impl Editor {
    fn new(previous: Option<TaskQuickAction>, name: &str, prompt: &str) -> Self {
        Self { previous, name: name.to_owned(), prompt: text_editor::Content::with_text(prompt) }
    }
}

fn message(event: Event) -> Message {
    Message::TaskQuickActions(domain::Event::Settings(event))
}

pub(crate) fn load(app: &mut App) -> Task<Message> {
    let Some(path) = super::project_path(app) else {
        return Task::none();
    };
    let origin = Origin::new(app, path);
    let state = &mut app.task_quick_actions.settings;
    if state.origin.as_ref() == Some(&origin) && state.busy {
        return Task::none();
    }
    *state = State { request: state.request, origin: Some(origin.clone()), ..State::default() };
    run(state, origin, false, catalog::list)
}

fn run(
    state: &mut State,
    origin: Origin,
    mutation: bool,
    operation: impl FnOnce(&std::path::Path) -> std::io::Result<Vec<TaskQuickAction>> + Send + 'static,
) -> Task<Message> {
    state.busy = true;
    state.saving = mutation;
    state.error = None;
    state.request = state.request.wrapping_add(1);
    let request = state.request;
    Task::perform(
        async move {
            let (_, project) = origin.resolve().await?;
            origin.catalog(&project, operation).await
        },
        move |result| message(Event::Finished { request, mutation, result }),
    )
}

pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    match event {
        Event::Load => return load(app),
        Event::Help(open) => {
            app.task_quick_actions.settings.help = open;
            return Task::none();
        }
        Event::Finished { request, .. } if request != app.task_quick_actions.settings.request => {
            return Task::none();
        }
        _ => {}
    }
    let Some(origin) = app.task_quick_actions.settings.origin.clone() else {
        return Task::none();
    };
    let path = super::project_path(app);
    if matches!(event, Event::Finished { .. }) && !origin.matches(app, path) {
        app.task_quick_actions.settings.busy = false;
        app.task_quick_actions.settings.saving = false;
        return Task::none();
    }
    if !origin.visible_in(app, path)
        || (matches!(event, Event::Save | Event::Archive(_)) && !origin.matches(app, path))
    {
        return domain::warning(app, "项目已切换，请重新加载快捷操作");
    }
    let locale = app.general_settings.desktop_locale;
    let state = &mut app.task_quick_actions.settings;
    if let Event::Finished { mutation, result, .. } = event {
        state.busy = false;
        state.saving = false;
        return match result {
            Ok(actions) => {
                state.actions = actions;
                state.editor = None;
                if mutation {
                    app.show_localized_toast(
                        crate::app::state::ToastKind::Success,
                        "已更新任务快捷操作",
                        vec![],
                    )
                } else {
                    Task::none()
                }
            }
            Err(error) => {
                state.error = Some(error);
                if mutation { domain::warning(app, error) } else { Task::none() }
            }
        };
    }
    if let Event::Search(query) = event {
        state.query = query;
        return Task::none();
    }
    if state.busy {
        return Task::none();
    }
    match event {
        Event::New => {
            state.error = None;
            state.editor = Some(Editor::new(None, "", ""));
            return operation::focus(NAME_INPUT);
        }
        Event::Example(index) => {
            if let Some(&(name, prompt)) = EXAMPLES.get(index) {
                let i18n = I18n::new(locale);
                state.error = None;
                state.editor = Some(Editor::new(None, i18n.__(name), i18n.__(prompt)));
                return operation::focus(NAME_INPUT);
            }
        }
        Event::Edit(id) => {
            if let Some(action) = state.actions.iter().find(|action| action.id == id) {
                state.error = None;
                state.editor =
                    Some(Editor::new(Some(action.clone()), &action.name, &action.prompt));
                return operation::focus(NAME_INPUT);
            }
        }
        Event::Archive(id) => {
            if let Some(action) = state.actions.iter().find(|action| action.id == id).cloned() {
                return run(state, origin, true, move |root| catalog::archive(root, &action));
            }
        }
        Event::Name(name) => {
            if let Some(editor) = &mut state.editor {
                editor.name = name;
            }
        }
        Event::Prompt(action) => {
            if let Some(editor) = &mut state.editor {
                editor.prompt.perform(action);
            }
        }
        Event::Save => {
            if let Some(editor) = &state.editor {
                let (name, prompt) = (editor.name.clone(), editor.prompt.text());
                if name.trim().is_empty() || prompt.trim().is_empty() {
                    return domain::warning(app, "请输入操作名称和提示词");
                }
                let previous = editor.previous.clone();
                return run(state, origin, true, move |root| {
                    catalog::save(root, previous.as_ref(), &name, &prompt)
                });
            }
        }
        Event::Close => state.editor = None,
        Event::Help(_) | Event::Search(_) | Event::Load | Event::Finished { .. } => {}
    }
    Task::none()
}

pub(crate) fn view(app: &App) -> Element<'_, Message> {
    let locale = app.general_settings.desktop_locale;
    let i18n = I18n::new(locale);
    let state = &app.task_quick_actions.settings;
    let path = super::project_path(app);
    let title = row![
        settings_page_heading(i18n.__(TITLE), i18n.__("保存当前项目常用的任务请求。")),
        settings_help_button_with_locale(locale, message(Event::Help(true))),
    ]
    .align_y(Alignment::Center)
    .spacing(8);
    let current = state.origin.as_ref().is_some_and(|origin| origin.visible_in(app, path));
    let chat_settings = Message::Batch(vec![
        Message::View(ViewMessage::OpenSystemSettingsTab(SystemTab::Agents)),
        Message::Settings(SettingsMessage::AgentConfigTab(AgentConfigTabMessage::Selected(
            AgentConfigTab::QuickMessages,
        ))),
    ]);
    let heading = row![
        container(title).width(Length::Fill),
        ui::action(i18n.__("聊天快捷消息"), Some(chat_settings)),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    let mut page = column![heading].spacing(18);
    if path.is_some() {
        page = page.push(catalog_view(i18n, state, current));
    }
    page.into()
}

pub(crate) fn overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let state = &app.task_quick_actions.settings;
    if state.help {
        let close = message(Event::Help(false));
        let modal = with_settings_help_modal(app, base, TITLE, HELP, close.clone());
        return desktop_ui_controls::system_settings_common::panels::with_settings_modal_escape(
            modal, close,
        );
    }
    if state.origin.as_ref().is_some_and(|origin| origin.visible_in(app, super::project_path(app)))
        && let Some(editor) = &state.editor
    {
        let i18n = I18n::new(app.general_settings.desktop_locale);
        return editor_modal(i18n, base, editor, !state.busy, state.error);
    }
    base
}

fn catalog_view(i18n: I18n, state: &State, current: bool) -> Element<'_, Message> {
    let enabled = !state.busy;
    let reload = || ui::action(i18n.__("加载快捷操作"), enabled.then_some(message(Event::Load)));
    if !current {
        return reload().into();
    }
    let search = settings_text_input(i18n.__("搜索快捷操作"), &state.query)
        .on_input(|query| message(Event::Search(query)));
    let create =
        ui::primary(i18n.__("新建操作"), Some(Icon::Plus), enabled.then_some(message(Event::New)));
    let mut content = column![ui::toolbar(search.into(), create)].spacing(12);
    if let Some(error) = state.error {
        content = content.push(
            row![container(settings_error_banner(i18n.__(error))).width(Length::Fill), reload(),]
                .spacing(8)
                .align_y(Alignment::Center),
        );
    }

    let query = state.query.trim().to_lowercase();
    let mut list = column![ui::record(
        [(ui::cell(i18n.__("名称")).into(), 2), (ui::cell(i18n.__("提示词")).into(), 3),],
        None
    )];
    let mut has_results = false;
    for item in &state.actions {
        if !ui::matches_search(&item.name, &query) && !ui::matches_search(&item.prompt, &query) {
            continue;
        }
        has_results = true;
        let name = row![ui::glyph(Icon::ArrowRepeat, 15.0), text(&item.name).size(14),]
            .spacing(8)
            .align_y(Alignment::Center);
        let menu = ui::menu(
            vec![
                (i18n.__("编辑"), message(Event::Edit(item.id))),
                (i18n.__("归档"), message(Event::Archive(item.id))),
            ],
            enabled,
        );
        list = list.push(settings_divider()).push(ui::record(
            [(name.into(), 2), (ui::cell(ui::preview(&item.prompt)).into(), 3)],
            Some(menu),
        ));
    }

    if !has_results {
        let label = if !enabled {
            "正在加载快捷操作…"
        } else if query.is_empty() {
            "尚未保存操作，可从示例开始。"
        } else {
            "没有匹配的结果"
        };
        let mut empty = column![ui::cell(i18n.__(label))].spacing(10);
        if enabled && state.actions.is_empty() && query.is_empty() {
            let examples = EXAMPLES.iter().enumerate().map(|(index, (name, _))| {
                ui::action(i18n.__(name), Some(message(Event::Example(index)))).into()
            });
            empty = empty.push(row(examples).spacing(8).wrap());
        }
        list = list.push(settings_divider()).push(container(empty).padding([18, 0]));
    }
    content.push(settings_group_card(list)).into()
}

fn editor_modal<'a>(
    i18n: I18n,
    base: Element<'a, Message>,
    editor: &'a Editor,
    enabled: bool,
    error: Option<&'static str>,
) -> Element<'a, Message> {
    let title = if editor.previous.is_some() {
        "编辑任务快捷操作"
    } else {
        "新建任务快捷操作"
    };
    let valid =
        enabled && !editor.name.trim().is_empty() && !editor.prompt.text().trim().is_empty();
    let mut body = column![
        settings_responsive_row(i18n.__("操作名称"), "", move || {
            let input = settings_text_input(i18n.__("例如：总结进展"), &editor.name).id(NAME_INPUT);
            if enabled {
                input
                    .on_input(|name| message(Event::Name(name)))
                    .on_submit_maybe(valid.then_some(message(Event::Save)))
            } else {
                input
            }
        }),
        settings_divider(),
        settings_responsive_row(
            i18n.__("提示词"),
            i18n.__("运行时先预览，再打开可编辑的会话草稿。"),
            move || {
                let input = settings_text_editor(&editor.prompt)
                    .height(162)
                    .placeholder(i18n.__("填写任务详情中要预览的提示词"));
                if enabled {
                    input.on_action(|action| message(Event::Prompt(action)))
                } else {
                    input
                }
            }
        ),
    ]
    .spacing(8);
    if let Some(error) = error {
        body = body.push(settings_error_banner(i18n.__(error)));
    }
    ui::modal(
        i18n,
        base,
        i18n.__(title),
        body,
        415.0,
        message(Event::Close),
        valid.then_some(message(Event::Save)),
    )
}
