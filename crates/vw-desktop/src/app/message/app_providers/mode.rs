use super::list::{self, ListMessage};
use crate::app::{
    App, Message,
    state::app_providers::{
        list::Scope,
        mode::{Form, Operation, SCROLL_ID},
    },
};
use iced::Task;
use vw_gateway_client::{
    AgentAppsReadError, GatewayClient,
    vw_api_types::{
        agent_apps::AppId,
        app_providers::{
            mode_apply::{ModeApplyAction, ModeApplyRequest, TargetPublication},
            mode_state::{ModeStateQuery, ProviderMode},
        },
    },
};

use crate::app::state::app_providers::mode::Outcome;

#[derive(Clone, Debug)]
pub enum Event {
    Open,
    Close,
    Refresh,
    ReviewDirect,
    Cancel,
    Confirm,
    Completed { scope: Scope, generation: u64, result: Result<Outcome, AgentAppsReadError> },
}

pub(crate) fn available(app: AppId) -> bool {
    cfg!(target_os = "macos") && app == AppId::Claude
}

pub(crate) fn message(event: Event) -> Message {
    list::message(ListMessage::Mode(event))
}

pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let state = &mut app.app_providers;
    let Some(scope) = state.scope.clone() else { return Task::none() };
    if matches!(event, Event::Open) {
        if !available(scope.app)
            || state.loading
            || state.mode.is_some()
            || state.form.is_some()
            || state.editor.is_some()
            || state.env.is_some()
            || state.preview.is_some()
            || state.snippets.is_some()
            || state.universal.is_some()
            || state.icons.is_some()
            || state.endpoints.is_some()
            || state.desktop.is_some()
        {
            return Task::none();
        }
        state.mode = Some(Form::default());
        return update(app, Event::Refresh);
    }
    let Some(form) = &mut state.mode else { return Task::none() };
    if matches!(event, Event::Close) {
        if !form.writing() {
            state.mode = None;
        }
        return Task::none();
    }
    if let Event::Completed { scope: completed, generation, .. } = &event
        && (*completed != scope || form.pending.is_none_or(|(active, _)| active != *generation))
    {
        return Task::none();
    }

    let (client, current) = match list::current(app, scope.app) {
        Ok(context) => context,
        Err(_) => {
            app.app_providers.mode = None;
            return app.show_error_toast("无法连接应用执行主机");
        }
    };
    let state = &mut app.app_providers;
    if current != scope {
        state.mode = None;
        return Task::none();
    }
    let Some(form) = &mut state.mode else { return Task::none() };
    let work = match event {
        Event::Refresh if form.pending.is_none() => {
            form.confirming_direct = false;
            form.status = None;
            Operation::Read
        }
        Event::ReviewDirect if form.can_restore_direct() => {
            form.confirming_direct = true;
            form.error = None;
            return scroll_top();
        }
        Event::Cancel if form.pending.is_none() => {
            form.confirming_direct = false;
            return Task::none();
        }
        Event::Confirm if form.confirming_direct && form.can_restore_direct() => {
            Operation::RestoreDirect
        }
        Event::Completed { generation, result, .. } => {
            if let Some(result) = form.finish(generation, result) {
                let toast = match result.target_publication {
                    TargetPublication::Published | TargetPublication::Unavailable => {
                        app.show_success_toast("已恢复直连配置")
                    }
                    TargetPublication::Pending => {
                        app.show_error_toast("已恢复直连配置，路由目标更新仍待完成")
                    }
                };
                return Task::batch([toast, list::update(app, ListMessage::Refresh)]);
            }
            if let Some(error) = form.error {
                return Task::batch([app.show_error_toast(error), scroll_top()]);
            }
            return Task::none();
        }
        _ => return Task::none(),
    };

    state.generation = state.generation.wrapping_add(1);
    let generation = state.generation;
    form.error = None;
    form.pending = Some((generation, work));
    let token = app.apps_catalog.token.clone();
    Task::perform(execute(client, scope.clone(), work, token), move |result| {
        message(Event::Completed { scope: scope.clone(), generation, result })
    })
}

async fn execute(
    client: GatewayClient,
    scope: Scope,
    work: Operation,
    token: String,
) -> Result<Outcome, AgentAppsReadError> {
    match work {
        Operation::Read => client
            .provider_mode_state(
                scope.app,
                &ModeStateQuery {
                    target_host_id: scope.host,
                    provider_id: None,
                    mode: ProviderMode::Direct,
                },
                &token,
            )
            .await
            .map(Outcome::Observed),
        Operation::RestoreDirect => client
            .provider_mode_apply(
                scope.app,
                &ModeApplyRequest {
                    target_host_id: scope.host,
                    action: ModeApplyAction::Direct {},
                },
                &token,
            )
            .await
            .map(Outcome::Applied),
    }
}

fn scroll_top() -> Task<Message> {
    iced::widget::operation::snap_to(
        iced::widget::Id::new(SCROLL_ID),
        iced::widget::scrollable::RelativeOffset::START,
    )
    .map(|(): ()| Message::None)
}
