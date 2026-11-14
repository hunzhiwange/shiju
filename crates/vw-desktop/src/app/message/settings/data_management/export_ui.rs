//! Portable export uses gateway authorization; only explicit client destinations are written.
use crate::app::{App, Message, state::AgentAppsContext};
use crate::app::message::SettingsMessage;
use iced::Task;
use vw_gateway_client::vw_api_types::{ProjectScope, agent_apps::data_management::*};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Identity {
    pub(super) context: AgentAppsContext,
    pub(super) project: String,
    pub(super) host: String,
    catalog: u64,
}
#[derive(Clone, Debug)]
pub(crate) enum Event {
    Open, Close, Principal(String), Select(DataCategory, bool), Export,
    Downloaded(Identity, u64, Result<Vec<u8>, String>),
    Finished(Identity, u64, Result<bool, String>),
}
#[derive(Clone, Default)]
pub(crate) struct State {
    pub open: bool,
    pub busy: bool,
    pub principal: String,
    pub categories: Vec<DataCategory>,
    identity: Option<Identity>,
    generation: u64,
}
pub(crate) fn message(event: Event) -> Message { Message::Settings(SettingsMessage::DataExport(event)) }
pub(crate) fn identity(app: &App) -> Option<Identity> {
    let client = crate::app::config::gateway_client().ok()?;
    let context = AgentAppsContext { gateway: client.endpoint().base_url(), tenant: client.endpoint().tenant_scope().clone(), project: app.project_path.clone() };
    if app.apps_catalog.context.as_ref() != Some(&context) || !app.apps_catalog.connection_open
        || app.apps_catalog.host.trim().is_empty() || app.apps_catalog.token.is_empty() { return None; }
    Some(Identity { context, project: app.project_id.clone().filter(|p| !p.trim().is_empty())?, host: app.apps_catalog.host.clone(), catalog: app.apps_catalog.generation })
}
pub(crate) fn visible(app: &App) -> bool {
    app.apps_catalog.data_export.open && identity(app).as_ref() == app.apps_catalog.data_export.identity.as_ref()
        && identity(app).is_some()
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let Some(current) = identity(app) else {
        app.apps_catalog.data_export = State::default();
        return Task::none();
    };
    let state = &mut app.apps_catalog.data_export;
    if matches!(event, Event::Open) {
        if state.busy && state.identity.as_ref() == Some(&current) { return Task::none(); }
        let generation = state.generation.wrapping_add(1);
        *state = State { open: true, identity: Some(current), generation, ..Default::default() };
        return Task::none();
    }
    if state.identity.as_ref() != Some(&current) || !state.open { return Task::none(); }
    match event {
        Event::Close if !state.busy => state.open = false,
        Event::Principal(value) if !state.busy => state.principal = value,
        Event::Select(category, selected) if !state.busy => {
            state.categories.retain(|entry| *entry != category);
            if selected { state.categories.push(category); }
        }
        Event::Export if !state.busy && !state.categories.is_empty() && !state.principal.trim().is_empty() => {
            let Ok(client) = crate::app::config::gateway_client() else { return Task::none(); };
            let Ok(scope) = ProjectScope::new(current.context.tenant.clone(), current.project.clone().into()) else { return Task::none(); };
            state.busy = true;
            state.generation = state.generation.wrapping_add(1);
            let generation = state.generation;
            let request = ExportDataRequest {
                target: DataManagementTarget { scope, target_host_id: current.host.clone(), principal_id: state.principal.trim().to_owned() },
                selections: state.categories.iter().map(|category| DataSelection { category: *category, entries: DataEntrySelection::All }).collect(),
                format: DataArchiveFormat::Json,
                idempotency_key: format!("desktop-export-{}-{generation}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()),
            };
            let token = app.apps_catalog.token.clone();
            return Task::perform(async move {
                let result = async {
                    let transfer = client.data_migration_export(&request, Some(&token)).await.map_err(|e| e.to_string())?;
                    client.data_migration_download(&request.target, &transfer.transfer_id, Some(&token)).await.map_err(|e| e.to_string())
                }.await;
                message(Event::Downloaded(current, generation, result))
            }, |message| message);
        }
        Event::Downloaded(request, generation, result) if request == current && generation == state.generation && state.busy => {
            let bytes = match result {
                Ok(bytes) => bytes,
                Err(error) => { state.busy = false; return app.show_error_toast(error); }
            };
            return Task::perform(async move {
                let destination = rfd::AsyncFileDialog::new().set_title("保存脱敏配置导出")
                    .set_file_name("识矩配置导出.json").add_filter("JSON", &["json"]).save_file().await;
                let result = tokio::task::spawn_blocking(move || save_download(destination.map(|f| f.path().to_owned()), &bytes))
                    .await.unwrap_or_else(|_| Err("保存任务失败".into()));
                message(Event::Finished(current, generation, result))
            }, |message| message);
        }
        Event::Finished(request, generation, result) if request == current && generation == state.generation && state.busy => {
            state.busy = false;
            return match result {
                Ok(true) => { state.open = false; app.show_success_toast("脱敏配置已保存到所选文件") }
                Ok(false) => Task::none(),
                Err(error) => app.show_error_toast(error),
            };
        }
        _ => {}
    }
    Task::none()
}

pub(super) fn save_download(destination: Option<std::path::PathBuf>, bytes: &[u8]) -> Result<bool, String> {
    use std::io::Write;
    let Some(path) = destination else { return Ok(false); };
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    { use std::os::unix::fs::OpenOptionsExt; options.mode(0o600); }
    let mut file = options.open(&path).map_err(|error| if error.kind() == std::io::ErrorKind::AlreadyExists {
        "目标已存在，请选择新文件名；不会覆盖已有文件".to_owned()
    } else { "无法创建所选文件，请检查目录权限或磁盘空间".to_owned() })?;
    if file.write_all(bytes).and_then(|_| file.sync_all()).is_err() {
        drop(file);
        return Err(if std::fs::remove_file(&path).is_ok() { "保存失败，已清理未完成文件" } else { "保存失败，未完成文件清理失败，请检查所选位置" }.into());
    }
    Ok(true)
}
