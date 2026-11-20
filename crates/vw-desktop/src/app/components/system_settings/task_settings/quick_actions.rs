use crate::app::components::system_settings_common::{
    primary_action_btn_style, rounded_action_btn_style, settings_close_button,
    settings_error_banner, settings_field_label, settings_help_button_with_locale,
    settings_modal_header, settings_modal_layout, settings_muted_text_style,
    settings_page_heading, settings_text_editor, settings_text_input, with_settings_help_modal,
};
use crate::app::i18n::I18n;
use crate::app::message::task_quick_actions::{self as domain, Origin};
use crate::app::message::{SettingsMessage, ViewMessage, settings::AgentConfigTabMessage};
use crate::app::state::AgentConfigTab;
use crate::app::{App, Message};
use desktop_settings_shell::SystemTab;
use desktop_ui_controls::system_settings_common::panels::settings_modal_overlay_with_escape;
use iced::widget::{button, column, row, scrollable, text, text_editor};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::{Alignment, Element, Length, Task};
use task_file_store::task_quick_actions::{self as catalog, TaskQuickAction};

const TITLE: &str = "任务快捷操作";
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
    Prompt(text_editor::Action),
    Save,
    Close,
    Help(bool),
    Finished {
        request: u64,
        mutation: bool,
        result: Result<Vec<TaskQuickAction>, &'static str>,
    },
}

#[derive(Default)]
pub(crate) struct State {
    origin: Option<Origin>,
    request: u64,
    busy: bool,
    actions: Vec<TaskQuickAction>,
    editor: Option<Editor>,
    error: Option<&'static str>,
    help: bool,
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
    if !origin.visible_in(app, path)
        || (matches!(event, Event::Save | Event::Archive(_) | Event::Finished { .. })
            && !origin.matches(app, path))
    {
        return domain::warning(app, "项目已切换，请重新加载快捷操作");
    }
    let locale = app.general_settings.desktop_locale;
    let state = &mut app.task_quick_actions.settings;
    if let Event::Finished { mutation, result, .. } = event {
        state.busy = false;
        return match result {
            Ok(actions) => {
                state.actions = actions;
                state.editor = None;
                if mutation {
                    app.show_localized_toast(crate::app::state::ToastKind::Success, "已更新任务快捷操作", vec![])
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
    if state.busy {
        return Task::none();
    }
    match event {
        Event::New => state.editor = Some(Editor::new(None, "", "")),
        Event::Example(index) => {
            if let Some(&(name, prompt)) = EXAMPLES.get(index) {
                let i18n = I18n::new(locale);
                state.editor = Some(Editor::new(None, i18n.__(name), i18n.__(prompt)));
            }
        }
        Event::Edit(id) => {
            if let Some(action) = state.actions.iter().find(|action| action.id == id) {
                state.editor = Some(Editor::new(Some(action.clone()), &action.name, &action.prompt));
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
        Event::Help(_) | Event::Load | Event::Finished { .. } => {}
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
        Message::Settings(SettingsMessage::AgentConfigTab(AgentConfigTabMessage::Selected(AgentConfigTab::QuickMessages))),
    ]);
    let mut page = column![title, action(i18n.__("聊天快捷消息"), Some(chat_settings))].spacing(12);
    if let Some(path) = path {
        page = page.push(text(path).size(12).style(settings_muted_text_style));
        page = page.push(catalog_view(i18n, state, current));
    } else {
        page = page.push(text(i18n.__("请先选择项目")));
    }

    let base = scrollable(desktop_settings_shell::centered_content(page))
        .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
        .height(Length::Fill)
        .into();
    if state.help {
        return with_settings_help_modal(app, base, i18n.__(TITLE), i18n.__(HELP), message(Event::Help(false)));
    }
    if current && let Some(editor) = &state.editor {
        return editor_modal(i18n, base, editor, !state.busy);
    }
    base
}

fn catalog_view(i18n: I18n, state: &State, current: bool) -> Element<'_, Message> {
    let reload = || action(i18n.__("加载快捷操作"), Some(message(Event::Load)));
    if !current {
        return reload().into();
    }
    if state.busy && state.actions.is_empty() && state.editor.is_none() {
        return text(i18n.__("正在加载快捷操作…")).into();
    }

    let mut items = column![].spacing(8);
    if let Some(error) = state.error {
        items = items.push(settings_error_banner(i18n.__(error))).push(reload());
    }
    let enabled = !state.busy;
    items = items.push(action(i18n.__("新建操作"), enabled.then_some(message(Event::New))));
    if state.actions.is_empty() {
        items = items.push(text(i18n.__("尚未保存操作，可从示例开始。")).size(13));
        for (index, &(name, _)) in EXAMPLES.iter().enumerate() {
            items = items.push(action(i18n.__(name), enabled.then_some(message(Event::Example(index)))));
        }
    }
    for item in &state.actions {
        items = items.push(row![
            text(&item.name).width(Length::Fill),
            action(i18n.__("编辑"), enabled.then_some(message(Event::Edit(item.id)))),
            action(i18n.__("归档"), enabled.then_some(message(Event::Archive(item.id)))),
        ].align_y(Alignment::Center).spacing(8));
    }
    items.into()
}

fn action(label: &str, on_press: Option<Message>) -> iced::widget::Button<'_, Message> {
    button(text(label).size(13))
        .padding([8, 12])
        .style(rounded_action_btn_style)
        .on_press_maybe(on_press)
}

fn editor_modal<'a>(
    i18n: I18n,
    base: Element<'a, Message>,
    editor: &'a Editor,
    enabled: bool,
) -> Element<'a, Message> {
    let close = message(Event::Close);
    let title = if editor.previous.is_some() { "编辑任务快捷操作" } else { "新建任务快捷操作" };
    let header = settings_modal_header(
        text(i18n.__(title)).size(18),
        settings_close_button(close.clone()),
    );
    let body = column![
        settings_field_label(i18n.__("操作名称")),
        settings_text_input(i18n.__("例如：总结进展"), &editor.name).on_input(|name| message(Event::Name(name))),
        settings_field_label(i18n.__("提示词")),
        settings_text_editor(&editor.prompt).height(180).on_action(|action| message(Event::Prompt(action))),
    ].spacing(8);
    let save = button(text(i18n.__(if enabled { "保存" } else { "正在保存…" })).size(13))
        .padding([8, 12])
        .style(primary_action_btn_style)
        .on_press_maybe(enabled.then_some(message(Event::Save)));
    let footer = row![action(i18n.__("取消"), enabled.then_some(close.clone())), save].spacing(8);
    let card = settings_modal_layout(header, body, Some(footer.into()))
        .width(Length::Fill)
        .max_width(560)
        .height(Length::Fill)
        .max_height(460);
    settings_modal_overlay_with_escape(base, close, card)
}
