use super::list::{self, ListMessage};
use crate::app::{
    App, Message,
    components::system_settings_apps::local,
    state::app_providers::{list::Scope, paths::Form},
};
use iced::Task;
use vw_gateway_client::{
    AgentAppsReadError, GatewayClient,
    vw_api_types::{agent_apps::AppId, app_providers::paths::*},
};

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

pub(crate) fn available(app: AppId) -> bool {
    cfg!(target_os = "macos") && app == AppId::Claude
}

pub(crate) fn message(event: Event) -> Message {
    list::message(ListMessage::Paths(event))
}

pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let state = &mut app.app_providers;
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
        return update(app, Event::Refresh);
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
    if let Event::Completed { catalog_generation, .. }
    | Event::Picked { catalog_generation, .. } = &event
        && *catalog_generation != app.apps_catalog.generation
    {
        state.paths = None;
        return Task::none();
    }
    let (client, current) = match list::current(app, scope.app) {
        Ok(context) if context.1 == scope => context,
        _ => {
            app.app_providers.paths = None;
            return Task::none();
        }
    };
    let state = &mut app.app_providers;
    let Some(form) = &mut state.paths else { return Task::none() };
    form.local_open =
        client.endpoint().is_loopback() && scope.host == vw_shared::local_apps::HOST_ID;
    let (action, opening_folder) = match event {
        Event::PickFolder if form.pending.is_none() && form.local_open => {
            state.generation = state.generation.wrapping_add(1);
            let generation = state.generation;
            form.pending = Some(generation);
            form.error = None;
            let catalog_generation = app.apps_catalog.generation;
            let token = app.apps_catalog.token.clone();
            return Task::perform(pick_local(client, scope.clone(), token), move |result| {
                message(Event::Picked {
                    scope: scope.clone(),
                    generation,
                    catalog_generation,
                    result,
                })
            });
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
                Err(()) => return app.show_error_toast("无法选择本机配置目录。"),
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
                    return app.show_error_toast("无法在本机打开配置目录。");
                };
                let token = app.apps_catalog.token.clone();
                return Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            let home = local::launch_directory(&client, &scope.host, &token, None)
                                .ok_or(())?;
                            open_local(&home, &status.paths.directory)
                        })
                        .await
                        .ok()
                        .and_then(Result::ok)
                    },
                    |opened| {
                        if opened.is_some() { Message::None } else { message(Event::FolderFailed) }
                    },
                );
            }
            if form.finish(generation, result) {
                return app.show_success_toast("已保存配置目录");
            }
            return Task::none();
        }
        Event::FolderFailed => return app.show_error_toast("无法在本机打开配置目录。"),
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
    let token = app.apps_catalog.token.clone();
    let catalog_generation = app.apps_catalog.generation;
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

async fn pick_local(
    client: GatewayClient,
    scope: Scope,
    token: String,
) -> Result<Option<String>, ()> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        // A loopback URL alone does not prove this gateway belongs to the local user.
        let home = tokio::task::spawn_blocking(move || {
            local::launch_directory(&client, &scope.host, &token, None).ok_or(())
        })
        .await
        .map_err(|_| ())??;
        let selected = rfd::AsyncFileDialog::new().set_directory(home).pick_folder().await;
        selected.map(|file| file.path().to_str().map(str::to_owned).ok_or(())).transpose()
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (client, scope, token);
        Err(())
    }
}

async fn execute(
    client: GatewayClient,
    scope: Scope,
    token: String,
    action: Option<DirectoryOverride>,
) -> Result<ProviderDirectoryStatus, AgentAppsReadError> {
    if let Some(action) = action {
        return client
            .app_provider_directory_save(
                scope.app,
                &ProviderDirectoryUpdate { target_host_id: scope.host, action },
                &token,
            )
            .await;
    }
    client
        .app_provider_directory(
            scope.app,
            &ProviderDirectoryQuery { target_host_id: scope.host },
            &token,
        )
        .await
}

#[cfg(target_os = "macos")]
fn open_local(home: &str, path: &str) -> Result<(), ()> {
    use std::path::Path;
    let home = Path::new(home);
    let path = Path::new(path);
    let relative = path.strip_prefix(home).map_err(|_| ())?;
    if relative.as_os_str().is_empty()
        || !relative.components().all(|part| matches!(part, std::path::Component::Normal(_)))
    {
        return Err(());
    }
    let actual = path.canonicalize().map_err(|_| ())?;
    if actual != home.canonicalize().map_err(|_| ())?.join(relative) || !actual.is_dir() {
        return Err(());
    }
    let status = std::process::Command::new("/usr/bin/open")
        .args(["-a", "Finder"])
        .arg(actual)
        .status()
        .map_err(|_| ())?;
    status.success().then_some(()).ok_or(())
}

#[cfg(not(target_os = "macos"))]
fn open_local(_: &str, _: &str) -> Result<(), ()> {
    Err(())
}
