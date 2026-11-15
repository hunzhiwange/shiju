use crate::{Scope,state::{paths::Form}};
use super::provider_update::{State,Effect as Message};
use iced::Task;
use {gateway_http::AgentAppsReadError, vw_gateway_client::GatewayClient, vw_api_types::{agent_apps::AppId}};
use vw_api_app_provider_environment::paths::*;

#[derive(Clone, Debug)]
pub enum Event {
    Open,
    Close,
    Refresh,
    Draft(String),
    Save,
    PickFolder,
    Picked {
        scope: Scope,
        generation: u64,
        catalog_generation: u64,
        result: Result<Option<String>, ()>,
    },
    OpenFolder,
    Completed {
        scope: Scope,
        generation: u64,
        catalog_generation: u64,
        result: Result<ProviderDirectoryStatus, AgentAppsReadError>,
        opening_folder: bool,
    },
    FolderFailed,
}

pub fn available(app: AppId) -> bool {
    cfg!(target_os = "macos") && app == AppId::Claude
}

pub fn message(event: Event) -> Message {
    Message::Paths(event)
}

pub fn update(state: &mut State, connection: Result<(GatewayClient,Scope),String>, token: &str, catalog_generation: u64, event: Event) -> Task<Message> {
    let Some(scope) = state.scope.clone() else { return Task::none() };
    if matches!(event, Event::Open) {
        if !available(scope.app)
            || state.loading
            || state.paths.is_some()
            || state.mode.is_some()
            || state.env.is_some()
            || state.preview.is_some()
            || state.form.is_some()
            || state.editor.is_some()
            || state.snippets.is_some()
            || state.universal.is_some()
            || state.icons.is_some()
            || state.endpoints.is_some()
            || state.desktop.is_some()
        {
            return Task::none();
        }
        state.paths = Some(Form::default());
        return update(state, connection.clone(), &token, catalog_generation, Event::Refresh);
    }
    let Some(form) = &mut state.paths else { return Task::none() };
    if matches!(event, Event::Close) {
        if !form.saving {
            state.paths = None;
        }
        return Task::none();
    }
    if let Event::Completed { scope: completed, generation, .. }
    | Event::Picked { scope: completed, generation, .. } = &event
        && (*completed != scope || form.pending != Some(*generation))
    {
        return Task::none();
    }
    if let Event::Completed { catalog_generation: received_catalog, .. }
    | Event::Picked { catalog_generation: received_catalog, .. } = &event
        && *received_catalog != catalog_generation
    {
        state.paths = None;
        return Task::none();
    }
    let (client, current) = match connection.clone() {
        Ok(context) if context.1 == scope => context,
        _ => {
            state.paths = None;
            return Task::none();
        }
    };
    let Some(form) = &mut state.paths else { return Task::none() };
    form.local_open =
        client.endpoint().is_loopback() && scope.host == local_app_auth::HOST_ID;
    let (action, opening_folder) = match event {
        Event::PickFolder if form.pending.is_none() && form.local_open => {
            state.generation = state.generation.wrapping_add(1);
            let generation = state.generation;
            form.pending = Some(generation);
            form.error = None;
            let token = token.to_owned();
            return Task::done(Message::PickFolder {client,scope,token,generation,catalog_generation});
        }
        Event::Picked { result, .. } => {
            form.pending = None;
            if !form.local_open {
                return Task::none();
            }
            match result {
                Ok(Some(path)) => {
                    form.draft = path;
                    form.error = None;
                }
                Ok(None) => {}
                Err(()) => return super::provider_update::error("无法选择本机配置目录。"),
            }
            return Task::none();
        }
        Event::Draft(value) if form.pending.is_none() => {
            form.draft = value;
            form.error = None;
            return Task::none();
        }
        Event::Completed { generation, result, opening_folder, .. } => {
            if opening_folder {
                form.pending = None;
                let Ok(status) = result else {
                    return super::provider_update::error("无法在本机打开配置目录。");
                };
                let token = token.to_owned();
                return Task::done(Message::OpenFolder {client,scope,token,path:status.paths.directory});
            }
            if form.finish(generation, result) {
                return super::provider_update::success("已保存配置目录");
            }
            return Task::none();
        }
        Event::FolderFailed => return super::provider_update::error("无法在本机打开配置目录。"),
        Event::Refresh if form.pending.is_none() => (None, false),
        Event::OpenFolder if form.pending.is_none() && form.paths.is_some() && form.local_open => {
            (None, true)
        }
        Event::Save if form.pending.is_none() => {
            let value = form.draft.trim();
            let action = if value.is_empty() {
                DirectoryOverride::Reset {}
            } else {
                DirectoryOverride::Set { directory: value.into() }
            };
            (Some(action), false)
        }
        _ => return Task::none(),
    };
    state.generation = state.generation.wrapping_add(1);
    let generation = state.generation;
    form.pending = Some(generation);
    form.saving = action.is_some();
    form.error = None;
    let token = token.to_owned();
    let work = execute(client, current.clone(), token, action);
    Task::perform(work, move |result| {
        message(Event::Completed {
            scope: current.clone(),
            generation,
            catalog_generation,
            result,
            opening_folder,
        })
    })
}

async fn execute(
    client: GatewayClient,
    scope: Scope,
    token: String,
    action: Option<DirectoryOverride>,
) -> Result<ProviderDirectoryStatus, AgentAppsReadError> {
    if let Some(action) = action {
        return gateway_app_provider::GatewayAppProviderClient::new(client.transport().clone()).app_provider_directory_save(
                scope.app,
                &ProviderDirectoryUpdate { target_host_id: scope.host, action },
                &token,
            )
            .await;
    }
    gateway_app_provider::GatewayAppProviderClient::new(client.transport().clone()).app_provider_directory(
            scope.app,
            &ProviderDirectoryQuery { target_host_id: scope.host },
            &token,
        )
        .await
}
