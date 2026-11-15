//! Portable export uses gateway authorization; only explicit client destinations are written.
use desktop_provider_apps::AgentAppsContext;
use iced::Task;
use {
    vw_api_transport::ProjectScope,
    vw_gateway_client::vw_api_app_operations::data_management::*,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Identity {
    pub context: AgentAppsContext,
    pub project: String,
    pub host: String,
    pub catalog: u64,
}
#[derive(Clone, Debug)]
pub enum Event {
    Feedback(bool, String),
    Open, Close, Principal(String), Select(DataCategory, bool), Export,
    Downloaded(Identity, u64, Result<Vec<u8>, String>),
    Finished(Identity, u64, Result<bool, String>),
}
#[derive(Clone, Default)]
pub struct State {
    pub open: bool,
    pub busy: bool,
    pub principal: String,
    pub categories: Vec<DataCategory>,
    identity: Option<Identity>,
    generation: u64,
}
pub enum Output { Event(Event), Error(String), Success(String) }
fn message(event: Event) -> Output { Output::Event(event) }
fn notify_error(value: impl Into<String>) -> Task<Output> { Task::done(Output::Error(value.into())) }
fn success(value: impl Into<String>) -> Task<Output> { Task::done(Output::Success(value.into())) }
pub fn visible(state: &State, current: Option<&Identity>) -> bool {
    state.open && current.is_some() && current == state.identity.as_ref()
}
pub fn update(state: &mut State, current: Option<Identity>, client: Option<vw_gateway_client::GatewayClient>, token: String, event: Event) -> Task<Output> {
    let Some(current) = current else {
        *state = State::default();
        return Task::none();
    };
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
            let Ok(client) = client.clone().ok_or(()) else { return Task::none(); };
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
            let token = token.clone();
            return Task::perform(async move {
                let result = async {
                    let transfer = gateway_app_data::GatewayAppDataClient::new(client.transport().clone()).data_migration_export(&request, Some(&token)).await.map_err(|e| e.to_string())?;
                    gateway_app_data::GatewayAppDataClient::new(client.transport().clone()).data_migration_download(&request.target, &transfer.transfer_id, Some(&token)).await.map_err(|e| e.to_string())
                }.await;
                message(Event::Downloaded(current, generation, result))
            }, |message| message);
        }
        Event::Downloaded(request, generation, result) if request == current && generation == state.generation && state.busy => {
            let bytes = match result {
                Ok(bytes) => bytes,
                Err(error) => { state.busy = false; return notify_error(error); }
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
                Ok(true) => { state.open = false; success("脱敏配置已保存到所选文件") }
                Ok(false) => Task::none(),
                Err(error) => notify_error(error),
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
