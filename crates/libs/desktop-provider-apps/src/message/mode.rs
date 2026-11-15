use crate::{Scope,state::{mode::{Form,Operation,SCROLL_ID}}};
use super::provider_update::{State,Effect as Message};
use iced::Task;
use {gateway_http::AgentAppsReadError, vw_gateway_client::GatewayClient, vw_api_types::{agent_apps::AppId}};
use vw_api_app_provider_environment::mode_apply::ModeApplyAction;
use vw_api_app_provider_environment::mode_apply::ModeApplyRequest;
use vw_api_app_provider_environment::mode_apply::TargetPublication;
use vw_api_app_provider_environment::mode_state::ModeStateQuery;
use vw_api_app_provider_environment::mode_state::ProviderMode;

use crate::state::mode::Outcome;

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

pub fn available(app: AppId) -> bool {
    cfg!(target_os = "macos") && app == AppId::Claude
}

pub fn message(event: Event) -> Message {
    Message::Mode(event)
}

pub fn update(state: &mut State, connection: Result<(GatewayClient,Scope),String>, token: &str, event: Event) -> Task<Message> {
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
        return update(state, connection.clone(), &token, Event::Refresh);
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

    let (client, current) = match connection.clone() {
        Ok(context) => context,
        Err(_) => {
            state.mode = None;
            return super::provider_update::error("无法连接应用执行主机");
        }
    };
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
                        super::provider_update::success("已恢复直连配置")
                    }
                    TargetPublication::Pending => {
                        super::provider_update::error("已恢复直连配置，路由目标更新仍待完成")
                    }
                };
                return Task::batch([toast, super::provider_update::refresh(state)]);
            }
            if let Some(error) = form.error {
                return Task::batch([super::provider_update::error(error), scroll_top()]);
            }
            return Task::none();
        }
        _ => return Task::none(),
    };

    state.generation = state.generation.wrapping_add(1);
    let generation = state.generation;
    form.error = None;
    form.pending = Some((generation, work));
    let token = token.to_owned();
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
        Operation::Read => gateway_app_provider::GatewayAppProviderClient::new(client.transport().clone()).provider_mode_state(
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
        Operation::RestoreDirect => gateway_app_provider::GatewayAppProviderClient::new(client.transport().clone()).provider_mode_apply(
                scope.app,
                &ModeApplyRequest {
                    target_host_id: scope.host,
                    project_id: None,
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
