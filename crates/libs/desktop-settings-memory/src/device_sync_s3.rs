//! Host-scoped S3 drafts. Secret values are write-only and never restored from responses.
use crate::system_settings_device_sync::task_panel::{TaskPanelMessage, TaskPanelState};
use super::data_management::export_ui::Identity;
use crate::system_settings_device_sync::preview::{PreviewMessage, PreviewState};
use iced::Task;
use {
    vw_api_transport::ProjectScope,
    vw_api_app_operations::DataManagementTarget,
    vw_api_device_sync::device_sync_config::*,
};
use gateway_device_sync::{DeviceSyncOperationsError, DeviceSyncSelection};

pub use crate::system_settings_device_sync::s3_modal::S3Preset;

#[derive(Clone)]
pub enum Event {
    Feedback(bool, String),
    Open,
    Close,
    Load,
    Save,
    Test,
    Principal(String),
    Selection(DeviceSyncSelection),
    Preset(S3Preset),
    Region(String),
    Bucket(String),
    AccessKey(String),
    Endpoint(String),
    Root(String),
    Profile(String),
    Secret(String),
    ClearSecret,
    Enabled(bool),
    AutoSync(bool),
    ConfirmAutoSync(bool),
    Preview(PreviewMessage),
    TaskPanel(TaskPanelMessage),
    Loaded(Identity, u64, Result<S3SyncConfigResponse, DeviceSyncOperationsError>),
    Saved(Identity, u64, Result<S3SyncConfigResponse, DeviceSyncOperationsError>),
    Tested(Identity, u64, Result<(), DeviceSyncOperationsError>),
}
impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("S3Event { [redacted] }")
    }
}
#[derive(Clone)]
pub struct State {
    pub open: bool,
    pub busy: bool,
    pub dirty: bool,
    pub principal: String,
    pub preset: S3Preset,
    pub draft: UpdateS3SyncConfigRequest,
    pub secret_touched: bool,
    pub secret_present: bool,
    pub auto_confirmation: bool,
    pub preview: PreviewState,
    pub task_panel: TaskPanelState,
    pub selection: DeviceSyncSelection,
    identity: Option<Identity>,
    generation: u64,
}
impl Default for State {
    fn default() -> Self {
        Self {
            open: false,
            busy: false,
            dirty: false,
            principal: String::new(),
            preset: S3Preset::Aws,
            draft: UpdateS3SyncConfigRequest {
                enabled: true,
                auto_sync: false,
                region: String::new(),
                bucket: String::new(),
                access_key_id: String::new(),
                endpoint: None,
                remote_root: String::new(),
                profile: String::new(),
                secret: SyncSecretUpdate::Clear {},
            },
            secret_touched: false,
            secret_present: false,
            auto_confirmation: false,
            preview: PreviewState::default(),
            task_panel: TaskPanelState::default(),
            selection: DeviceSyncSelection::default(),
            identity: None,
            generation: 0,
        }
    }
}
impl State {
    pub fn reset(&mut self) {
        let generation = self.generation.wrapping_add(1);
        *self = Self { generation, ..Self::default() };
    }
    fn edit(&mut self, connection: bool) {
        self.dirty = true;
        self.preview.invalidate();
        if connection {
            self.draft.secret = SyncSecretUpdate::Clear {};
            self.secret_touched = false;
            self.secret_present = false;
            self.draft.auto_sync = false;
            self.auto_confirmation = false;
        }
    }
    fn accept(&mut self, response: &S3SyncConfigResponse) {
        if let Some(c) = &response.config {
            self.draft = UpdateS3SyncConfigRequest {
                enabled: c.enabled,
                auto_sync: c.auto_sync,
                region: c.region.clone(),
                bucket: c.bucket.clone(),
                access_key_id: c.access_key_id.clone(),
                endpoint: c.endpoint.clone(),
                remote_root: c.remote_root.clone(),
                profile: c.profile.clone(),
                secret: SyncSecretUpdate::Keep {},
            };
            self.secret_present = c.secret_present;
        } else {
            self.draft = Self::default().draft;
            self.secret_present = false;
        }
        self.secret_touched = false;
        self.dirty = false;
        self.auto_confirmation = false;
        self.preview.invalidate();
    }
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
        state.reset();
        return Task::none();
    };
    let Ok(client) = client.clone().ok_or(()) else {
        return Task::none();
    };
    let client = gateway_device_sync::GatewayDeviceSyncClient::new(client.transport().clone());
    let token = token.clone();
    if matches!(event, Event::Open) || state.identity.as_ref() != Some(&current) {
        state.reset();
        state.identity = Some(current.clone());
        if !matches!(event, Event::Open) {
            return Task::none();
        }
        state.open = true;
    }
    if !state.open {
        return Task::none();
    }
    let saved_response = matches!(&event, Event::Saved(..));
    let blocked = state.busy || state.preview.busy || state.task_panel.is_busy();
    match event {
        Event::Open | Event::Load | Event::TaskPanel(TaskPanelMessage::Refresh) if !state.busy => {
            if state.dirty {
                return notify_error("请先保存 S3 草稿再刷新");
            }
            state.busy = true;
            state.generation = state.generation.wrapping_add(1);
            let generation = state.generation;
            return Task::perform(
                async move {
                    let result = client
                        .device_sync_s3_config(
                            &current.project.clone().into(),
                            &current.host,
                            &token,
                        )
                        .await;
                    message(Event::Loaded(current, generation, result))
                },
                |m| m,
            );
        }
        Event::Close if !state.busy => {
            state.open = false;
            state.draft.secret = if state.secret_present {
                SyncSecretUpdate::Keep {}
            } else {
                SyncSecretUpdate::Clear {}
            };
            state.secret_touched = false;
            state.preview.invalidate();
        }
        Event::Selection(v) if !blocked => {
            state.selection = v;
            state.preview.invalidate();
        }
        Event::Principal(v) if !blocked => {
            state.principal = v;
            state.edit(false);
            state.task_panel.reset();
        }
        Event::Preset(v) if !blocked => {
            state.preset = v;
            state.edit(true);
            state.draft.endpoint = None;
            state.draft.region = match v {
                S3Preset::R2 => "auto".into(),
                _ => String::new(),
            };
        }
        Event::Region(v) if !blocked => {
            state.draft.region = v;
            state.edit(true);
        }
        Event::Bucket(v) if !blocked => {
            state.draft.bucket = v;
            state.edit(true);
        }
        Event::AccessKey(v) if !blocked => {
            state.draft.access_key_id = v;
            state.edit(true);
        }
        Event::Endpoint(v) if !blocked => {
            state.draft.endpoint = if v.trim().is_empty() { None } else { Some(v) };
            state.edit(true);
        }
        Event::Root(v) if !blocked => {
            state.draft.remote_root = v;
            state.edit(true);
        }
        Event::Profile(v) if !blocked => {
            state.draft.profile = v;
            state.edit(true);
        }
        Event::Secret(v) if !blocked => {
            state.draft.secret = SyncSecretUpdate::Replace { value: v };
            state.secret_touched = true;
            state.edit(false);
        }
        Event::ClearSecret if !blocked => {
            state.draft.secret = SyncSecretUpdate::Clear {};
            state.secret_touched = true;
            state.edit(false);
        }
        Event::Enabled(v) if !blocked => {
            state.draft.enabled = v;
            state.edit(false);
        }
        Event::AutoSync(v) if !blocked => {
            state.draft.auto_sync = false;
            state.auto_confirmation = v;
            state.edit(false);
        }
        Event::ConfirmAutoSync(v) if !blocked => {
            state.draft.auto_sync = v && state.auto_confirmation;
            state.auto_confirmation = false;
            state.edit(false);
        }
        Event::Save | Event::Test if !blocked => {
            if state.draft.auto_sync && state.auto_confirmation {
                return notify_error("请明确确认自动同步");
            }
            if state.preset != S3Preset::Aws && state.draft.endpoint.is_none() {
                return notify_error("请填写 S3 兼容服务的 Endpoint");
            }
            let saving = matches!(event, Event::Save);
            let draft = state.draft.clone();
            state.busy = true;
            state.generation = state.generation.wrapping_add(1);
            let generation = state.generation;
            return Task::perform(
                async move {
                    if saving {
                        message(Event::Saved(
                            current.clone(),
                            generation,
                            client
                                .device_sync_save_s3_config(
                                    &current.project.clone().into(),
                                    &current.host,
                                    &draft,
                                    &token,
                                )
                                .await,
                        ))
                    } else {
                        message(Event::Tested(
                            current.clone(),
                            generation,
                            client
                                .device_sync_test_s3(
                                    &current.project.clone().into(),
                                    &current.host,
                                    &draft,
                                    &token,
                                )
                                .await,
                        ))
                    }
                },
                |m| m,
            );
        }
        Event::Loaded(request, generation, result) | Event::Saved(request, generation, result)
            if request == current && generation == state.generation && state.busy =>
        {
            state.busy = false;
            let response = match result {
                Ok(r) => r,
                Err(_) => return notify_error("S3 配置请求失败，草稿已保留"),
            };
            state.accept(&response);
            if saved_response {
                state.busy = true;
                let draft = state.draft.clone();
                return Task::perform(
                    async move {
                        message(Event::Tested(
                            current.clone(),
                            generation,
                            client
                                .device_sync_test_s3(
                                    &current.project.clone().into(),
                                    &current.host,
                                    &draft,
                                    &token,
                                )
                                .await,
                        ))
                    },
                    |m| m,
                );
            }
            let Ok(scope) =
                ProjectScope::new(current.context.tenant.clone(), current.project.clone().into())
            else {
                return Task::none();
            };
            let target = DataManagementTarget {
                scope,
                target_host_id: current.host,
                principal_id: "desktop-local-user".into(),
            };
            let (task, toast) = state.task_panel.update(
                TaskPanelMessage::Discover {
                    last_status: response.last_status,
                    active_task_id: response.active_task_id,
                },
                client,
                target,
                token,
            );
            let task = task.map(|e| message(Event::TaskPanel(e)));
            return match toast {
                Some(t) => Task::batch([task, notify_error(t)]),
                None => task,
            };
        }
        Event::Tested(request, generation, result)
            if request == current && generation == state.generation && state.busy =>
        {
            state.busy = false;
            let clean = !state.dirty;
            return match result {
                Ok(()) => Task::batch([
                    success("S3 连接测试成功"),
                    if clean { Task::done(message(Event::Load)) } else { Task::none() },
                ]),
                Err(_) => notify_error("S3 连接测试失败，请检查配置"),
            };
        }
        Event::Preview(e) if !state.busy && !state.task_panel.is_busy() => {

            let Ok(scope) =
                ProjectScope::new(current.context.tenant.clone(), current.project.clone().into())
            else {
                return Task::none();
            };
            let target = DataManagementTarget {
                scope,
                target_host_id: current.host,
                principal_id: "desktop-local-user".into(),
            };
            let (task, toast, status) = state.preview.update(
                e,
                client.clone(),
                target.clone(),
                token.clone(),
                DeviceSyncTransport::S3,
                state.selection.clone(),
                state.dirty,
            );
            let mut tasks = vec![task.map(|e| message(Event::Preview(e)))];
            if let Some(status) = status {
                let (task, _) = state.task_panel.update(
                    TaskPanelMessage::Discover {
                        active_task_id: Some(status.task_id.clone()),
                        last_status: Some(status),
                    },
                    client,
                    target,
                    token,
                );
                tasks.push(task.map(|e| message(Event::TaskPanel(e))));
            }
            if let Some(t) = toast {
                tasks.push(notify_error(t));
            }
            return Task::batch(tasks);
        }
        Event::TaskPanel(e) => {
            let Ok(scope) =
                ProjectScope::new(current.context.tenant.clone(), current.project.clone().into())
            else {
                return Task::none();
            };
            let target = DataManagementTarget {
                scope,
                target_host_id: current.host,
                principal_id: "desktop-local-user".into(),
            };
            let (task, toast) = state.task_panel.update(e, client, target, token);
            let task = task.map(|e| message(Event::TaskPanel(e)));
            return match toast {
                Some(t) => Task::batch([task, notify_error(t)]),
                None => task,
            };
        }
        _ => {}
    }
    Task::none()
}
