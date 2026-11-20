use crate::app::components::system_settings_common::{
    primary_action_btn_style, rounded_action_btn_style, settings_close_button,
    settings_modal_header, settings_modal_layout, settings_muted_text_style,
};
use crate::app::i18n::I18n;
use crate::app::message::task_quick_actions::{self as domain, Origin};
use crate::app::{App, Message, Screen};
use desktop_chat_input::input_panel::quick_actions::task_quick_action_draft;
use desktop_ui_controls::system_settings_common::panels::settings_modal_overlay_with_escape;
use iced::widget::{button, column, row, text};
use iced::{Element, Length, Task};
use task_file_store::task_quick_actions::{self as catalog, TaskQuickAction};
use vw_api_project::ProjectDto;
use vw_api_transport::ProjectId;

const TITLE: &str = "任务快捷操作";
const TASK_UNAVAILABLE: &str = "任务已不可访问，未打开快捷操作草稿";

#[derive(Clone, Debug)]
pub enum Event {
    Open { project_path: String, task_id: String },
    Loaded(u64, Result<Choices, &'static str>),
    Select(u64),
    Confirm,
    Confirmed(u64, Result<ProjectDto, &'static str>),
    Close,
}

#[derive(Default)]
pub(crate) struct State {
    request: u64,
    dialog: Option<Dialog>,
}

struct Dialog {
    origin: Origin,
    task_id: String,
    stage: Stage,
}

enum Stage {
    Loading,
    Choose(Choices),
    Preview {
        choices: Choices,
        action: TaskQuickAction,
        draft: String,
        verifying: bool,
    },
}

#[derive(Clone, Debug)]
pub struct Choices {
    project_id: ProjectId,
    context: Context,
    actions: Vec<TaskQuickAction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Context {
    title: Option<String>,
    description: String,
}

fn message(event: Event) -> Message {
    Message::TaskQuickActions(domain::Event::Board(event))
}

fn current(dialog: &Dialog, app: &App) -> bool {
    app.navigation_tabs.screen == Screen::TaskBoard
        && dialog.origin.matches(app, app.navigation_tabs.project_path.as_deref())
}

async fn task_context(
    origin: &Origin,
    task_id: &str,
) -> Result<(ProjectDto, Context), &'static str> {
    let (client, project) = origin.resolve().await?;
    let scope = desktop_session::session_gateway::active_agent_scheduler_scope(
        client.endpoint().clone(),
        project.id.clone(),
    ).map_err(|_| TASK_UNAVAILABLE)?;
    let snapshot = gateway_scheduler::GatewaySchedulerClient::new(client.transport().clone())
        .agent_scheduler_status(&scope)
        .await
        .map_err(|_| TASK_UNAVAILABLE)?;
    if snapshot.project_id != project.id {
        return Err(TASK_UNAVAILABLE);
    }
    let task = snapshot.tasks.into_iter()
        .find(|task| task.task_id.as_ref() == task_id && task.project_id == project.id)
        .ok_or(TASK_UNAVAILABLE)?;
    Ok((project, Context { title: task.read_model.title, description: task.prompt }))
}

fn open(app: &mut App, project_path: String, task_id: String) -> Task<Message> {
    let origin = Origin::new(app, &project_path);
    let dialog = Dialog { origin: origin.clone(), task_id: task_id.clone(), stage: Stage::Loading };
    if !current(&dialog, app) {
        return domain::warning(app, "项目已切换，未打开快捷操作草稿");
    }

    let state = &mut app.task_quick_actions.board;
    state.request = state.request.wrapping_add(1);
    let request = state.request;
    state.dialog = Some(dialog);
    Task::perform(
        async move {
            let (project, context) = task_context(&origin, &task_id).await?;
            let actions = origin.catalog(&project, catalog::list).await?;
            Ok(Choices { project_id: project.id, context, actions })
        },
        move |result| message(Event::Loaded(request, result)),
    )
}

pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    match event {
        Event::Open { project_path, task_id } => return open(app, project_path, task_id),
        Event::Close => {
            app.task_quick_actions.board.dialog = None;
            return Task::none();
        }
        Event::Loaded(request, _) | Event::Confirmed(request, _)
            if request != app.task_quick_actions.board.request => return Task::none(),
        _ => {}
    }

    let Some(dialog) = app.task_quick_actions.board.dialog.as_ref() else {
        return Task::none();
    };
    if !current(dialog, app) {
        app.task_quick_actions.board.dialog = None;
        return domain::warning(app, "项目或页面已切换，未打开快捷操作草稿");
    }

    let i18n = I18n::new(app.general_settings.desktop_locale);
    let state = &mut app.task_quick_actions.board;
    let Some(dialog) = &mut state.dialog else {
        return Task::none();
    };
    match event {
        Event::Loaded(_, result) => match result {
            Ok(choices) => dialog.stage = Stage::Choose(choices),
            Err(error) => {
                state.dialog = None;
                return domain::warning(app, error);
            }
        },
        Event::Select(id) => {
            if let Stage::Choose(choices) = &dialog.stage
                && let Some(action) = choices.actions.iter().find(|action| action.id == id)
            {
                let draft = task_quick_action_draft(
                    i18n,
                    &dialog.task_id,
                    choices.context.title.as_deref(),
                    &choices.context.description,
                    &action.prompt,
                );
                dialog.stage = Stage::Preview {
                    choices: choices.clone(),
                    action: action.clone(),
                    draft,
                    verifying: false,
                };
            }
        }
        Event::Confirm => {
            let Stage::Preview { choices, action, verifying, .. } = &mut dialog.stage else {
                return Task::none();
            };
            if *verifying {
                return Task::none();
            }
            *verifying = true;
            let (origin, task_id) = (dialog.origin.clone(), dialog.task_id.clone());
            let (project_id, context, action) =
                (choices.project_id.clone(), choices.context.clone(), action.clone());
            let request = state.request;
            return Task::perform(
                async move {
                    let (project, fresh_context) = task_context(&origin, &task_id).await?;
                    if project.id != project_id || fresh_context != context {
                        return Err("任务已更新，请重新预览快捷操作");
                    }
                    let actions = origin.catalog(&project, catalog::list).await?;
                    if !actions.contains(&action) {
                        return Err("快捷操作已更新或归档，请重新选择");
                    }
                    Ok(project)
                },
                move |result| message(Event::Confirmed(request, result)),
            );
        }
        Event::Confirmed(_, result) => {
            let Some(Dialog { origin, stage: Stage::Preview { draft, verifying: true, .. }, .. }) =
                state.dialog.take()
            else {
                return Task::none();
            };
            return match result {
                Ok(project) => crate::app::message::chat::session::open_task_quick_action_draft(
                    app, origin.path, project, draft,
                ),
                Err(error) => domain::warning(app, error),
            };
        }
        Event::Open { .. } | Event::Close => {}
    }
    Task::none()
}

pub(crate) fn entry<'a>(app: &App, task_id: &str) -> Option<Element<'a, Message>> {
    let project_path = app.navigation_tabs.project_path.clone()?;
    let i18n = I18n::new(app.general_settings.desktop_locale);
    Some(button(text(i18n.__(TITLE)).size(13))
        .padding([8, 12])
        .style(rounded_action_btn_style)
        .on_press(message(Event::Open { project_path, task_id: task_id.to_owned() }))
        .into())
}

pub(crate) fn view_overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let Some(dialog) = app.task_quick_actions.board.dialog.as_ref() else {
        return base;
    };
    if !dialog.origin.visible_in(app, app.navigation_tabs.project_path.as_deref()) {
        return base;
    }
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let close = message(Event::Close);
    let header = settings_modal_header(text(i18n.__(TITLE)).size(18), settings_close_button(close.clone()));
    let mut body = column![text(&dialog.task_id).size(13).style(settings_muted_text_style)].spacing(12);
    let mut footer = row![
        button(text(i18n.__("取消")).size(13))
            .padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press(close.clone())
    ].spacing(8);
    match &dialog.stage {
        Stage::Loading => body = body.push(text(i18n.__("正在加载快捷操作…"))),
        Stage::Choose(choices) => {
            if let Some(title) = &choices.context.title {
                body = body.push(text(title));
            }
            if choices.actions.is_empty() {
                body = body.push(text(i18n.__("请在「任务与工作流」设置中为当前项目添加快捷操作。")));
            }
            for action in &choices.actions {
                body = body.push(button(text(&action.name).size(13))
                    .padding([8, 12])
                    .style(rounded_action_btn_style)
                    .on_press(message(Event::Select(action.id))));
            }
        }
        Stage::Preview { action, draft, verifying, .. } => {
            body = column![
                text(&action.name).size(16),
                text(i18n.__("确认后将打开可编辑的草稿，需手动发送。")).size(13).style(settings_muted_text_style),
                text(draft),
            ].spacing(12);
            footer = footer.push(
                button(text(i18n.__(if *verifying { "正在核对任务…" } else { "打开会话草稿" })).size(13))
                    .padding([8, 12])
                    .style(primary_action_btn_style)
                    .on_press_maybe((!*verifying).then_some(message(Event::Confirm))),
            );
        }
    }
    let card = settings_modal_layout(header, body, Some(footer.into()))
        .width(Length::Fill)
        .max_width(640)
        .height(Length::Fill)
        .max_height(520);
    settings_modal_overlay_with_escape(base, close, card)
}
