//! Authenticated desktop management, with serialized edits and stale-response rejection.
use crate::computer_control_state::*;
use gateway_endpoint::GatewayEndpoint;
use iced::Task;
use vw_api_computer_control::*;

#[derive(Debug, Clone)]
pub enum Event {
    Refresh,
    Manage(ControlSetting),
    ManageClose,
    CopyNativeInstallPath,
    CopyNativeInstallCommand,
    OpenNativePermissionGuide { screen_recording: bool },
    OpenSystemPermission { screen_recording: bool },
    NativePermissionRequested { identity: ControlIdentity, generation: u64, result: Result<NativePermissionReport, String> },
    NativeServiceLoaded { identity: ControlIdentity, generation: u64, result: Result<NativeServiceReport, String> },
    Loaded { identity: ControlIdentity, generation: u64, result: Result<(ComputerControlStatus, ControlApprovals), String> },
    Toggle(ControlSetting, bool),
    Revoke(ControlGrantScope),
    Stop,
    Stopped { identity: ControlIdentity, result: Result<(), String> },
    Completed { identity: ControlIdentity, generation: u64, result: Result<(), String> },
}
pub fn message(event: Event) -> Event { event }

fn safe_error(error: gateway_computer_control::ComputerControlApiError) -> String {
    if error == gateway_computer_control::ComputerControlApiError::Http(401) {
        return "电脑操控鉴权失败（HTTP 401）：当前网关已启用鉴权，请检查网关连接设置中的 skey，然后刷新状态。".into();
    }
    // The client only exposes status/transport/decode errors, never response bodies.
    format!("电脑操控请求失败：{error}。请刷新后重试。")
}
fn refresh(state: &mut ComputerControlSettingsState, endpoint: &GatewayEndpoint) -> Task<Event> {
    if state.busy() { return Task::none(); }
    state.generation += 1;
    let generation = state.generation;
    state.loading = true;
    state.status = None;
    state.approvals.clear();
    state.error = None;
    let identity = ControlIdentity::from_endpoint(endpoint);
    state.identity = Some(identity.clone());
    state.native_service = None;
    state.native_permission_report = None;
    state.target_host = endpoint.base_url();
    state.target_is_local = endpoint.is_loopback();
    #[cfg(target_os = "macos")]
    if state.target_is_local {
        state.native_install = std::env::current_exe().ok().and_then(|path| NativeInstall::from_launcher(&path));
    }
    let client = gateway_http::GatewayTransport::new(endpoint.clone())
        .map(gateway_computer_control::GatewayComputerControlClient::new);
    let service_identity = identity.clone();
    let service_client = client.clone();
    let service_task = Task::perform(async move {
        service_client?.native_service().await.map_err(safe_error)
    }, move |result| Event::NativeServiceLoaded { identity: service_identity.clone(), generation, result });
    Task::batch([service_task, Task::perform(async move {
        let client = client?;
        let status = client.computer_control_status().await.map_err(safe_error)?;
        let approvals = client.computer_control_approvals().await.map_err(safe_error)?;
        Ok((status, approvals))
    }, move |result| message(Event::Loaded { identity: identity.clone(), generation, result }))])
}

pub fn update(state: &mut ComputerControlSettingsState, endpoint: &GatewayEndpoint, event: Event) -> (Task<Event>, Option<Notice>) {
    let mut notice = None;
    let task = update_inner(state, endpoint, event, &mut notice);
    (task, notice)
}

#[derive(Debug, Clone)]
pub enum Notice { Error(String), Success(String) }
fn notify(notice: &mut Option<Notice>, value: Notice) -> Task<Event> { *notice = Some(value); Task::none() }
fn update_inner(state: &mut ComputerControlSettingsState, endpoint: &GatewayEndpoint, event: Event, notice: &mut Option<Notice>) -> Task<Event> {
    let identity = ControlIdentity::from_endpoint(endpoint);
    if state.identity.as_ref() != Some(&identity) {
        let generation = state.generation + 1;
        *state = Default::default();
        state.generation = generation;
        state.identity = Some(identity.clone());
    }
    match event {
        Event::Manage(feature @ ControlSetting::NativeApps) => {
            state.management = Some(feature);
            Task::none()
        }
        Event::Manage(_) => Task::none(),
        Event::ManageClose => { state.management = None; Task::none() }
        Event::OpenNativePermissionGuide { .. } => Task::none(),
        event @ (Event::CopyNativeInstallPath | Event::CopyNativeInstallCommand) => {
            let Some(install) = state.native_install.as_ref().filter(|_| endpoint.is_loopback()) else {
                return Task::none();
            };
            let (value, label) = if matches!(event, Event::CopyNativeInstallPath) {
                (&install.app_path, "已复制安装地址")
            } else {
                (&install.command, "已复制安装命令")
            };
            *notice = Some(Notice::Success(label.into()));
            iced::clipboard::write(value.clone())
        }
        Event::OpenSystemPermission { screen_recording } => {
            if state.busy() { return Task::none(); }
            if !endpoint.is_loopback() {
                return notify(notice, Notice::Error("请在运行远程网关的电脑上申请系统权限".into()));
            }
            if !state.native_service.as_ref().is_some_and(|report| report.state == NativeServiceState::Available) {
                return notify(notice, Notice::Error("请先刷新并确认执行应用已安装且可连接".into()));
            }
            state.generation += 1;
            let generation = state.generation;
            state.requesting_permission = true;
            let client = gateway_http::GatewayTransport::new(endpoint.clone()).map(gateway_computer_control::GatewayComputerControlClient::new);
            Task::perform(async move { client?.request_native_permission(screen_recording).await.map_err(safe_error) }, move |result| Event::NativePermissionRequested { identity: identity.clone(), generation, result })
        }
        Event::NativePermissionRequested { identity: source, generation, result } => {
            if generation != state.generation || !state.requesting_permission || source != identity { return Task::none(); }
            state.requesting_permission = false;
            match result {
                Ok(report) => {
                    state.native_permission_report = Some(report);
                    refresh(state, endpoint)
                }
                Err(error) => notify(notice, Notice::Error(error)),
            }
        }
        Event::NativeServiceLoaded { identity: source, generation, result } => {
            if source != identity || generation != state.generation { return Task::none(); }
            match result {
                Ok(report) => { state.native_permission_report = report.permissions.clone(); state.native_service = Some(report); }
                Err(error) => return notify(notice, Notice::Error(error)),
            }
            Task::none()
        }
        Event::Refresh => refresh(state, endpoint),
        Event::Loaded { identity: source, generation, result } => {
            if source != identity { return Task::none(); }
            if !state.apply_loaded(generation, result) { return Task::none(); }
            if let Some(error) = state.error.clone() { return notify(notice, Notice::Error(error)); }
            Task::none()
        }
        Event::Stop => {
            if state.stopping { return Task::none(); }
            state.stopping = true;
            let client = gateway_http::GatewayTransport::new(endpoint.clone())
                .map(gateway_computer_control::GatewayComputerControlClient::new);
            Task::perform(async move { client?.computer_control_stop().await.map_err(safe_error) },
                move |result| message(Event::Stopped { identity: identity.clone(), result }))
        }
        Event::Stopped { identity: source, result } => {
            if source != identity { return Task::none(); }
            state.stopping = false;
            match result {
                Ok(()) => notify(notice, Notice::Success("服务端已停止电脑操控".into())),
                Err(error) => notify(notice, Notice::Error(error)),
            }
        }
        Event::Completed { identity: source, generation, result } => {
            if source != identity || generation != state.generation || !state.saving { return Task::none(); }
            state.saving = false;
            let toast = match result {
                Ok(()) => notify(notice, Notice::Success("服务端已确认授权更改".into())),
                Err(error) => notify(notice, Notice::Error(error)),
            };
            Task::batch([toast, refresh(state, endpoint)])
        }
        event @ (Event::Toggle(..) | Event::Revoke(_)) => {
            if state.busy() { return Task::none(); }
            let Some(status) = &state.status else { return refresh(state, endpoint); };
            let version = status.permission_version;
            state.generation += 1;
            let generation = state.generation;
            state.saving = true;
            state.error = None;
            let client = gateway_http::GatewayTransport::new(endpoint.clone())
                .map(gateway_computer_control::GatewayComputerControlClient::new);
            Task::perform(async move {
                let client = client?;
                match event {
                    Event::Toggle(feature, enabled) => {
                        client.computer_control_save(&ControlSettingsUpdate { permission_version: version, feature, enabled }).await.map(|_| ()).map_err(safe_error)
                    }
                    Event::Revoke(scope) => {
                        client.computer_control_revoke(&ControlGrantRevocation { permission_version: version, scope }).await.map(|_| ()).map_err(safe_error)
                    }
                    _ => unreachable!(),
                }
            }, move |result| message(Event::Completed { identity: identity.clone(), generation, result }))
        }
    }
}

#[cfg(test)]
#[path = "computer_control_update_tests.rs"]
mod tests;
