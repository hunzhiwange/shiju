//! Authenticated desktop management, with serialized edits and stale-response rejection.
use crate::computer_control_state::*;
use gateway_endpoint::GatewayEndpoint;
use iced::Task;
use vw_api_computer_control::*;

#[derive(Debug, Clone)]
pub enum Event {
    Refresh,
    Loaded { identity: ControlIdentity, generation: u64, result: Result<(ComputerControlStatus, ControlApprovals), String> },
    Toggle(ControlSetting, bool),
    Revoke(ControlGrantScope),
    Stop,
    Stopped { identity: ControlIdentity, result: Result<(), String> },
    PairOpen(ControlSetting),
    PairDocument(String),
    PairClose,
    PairConfirm,
    CopyCredential,
    Completed { identity: ControlIdentity, generation: u64, result: Result<Option<PairingCredential>, String> },
    CredentialExpired { identity: ControlIdentity, generation: u64 },
}
pub fn message(event: Event) -> Event { event }

fn safe_error(error: impl std::fmt::Display) -> String {
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
    let client = gateway_http::GatewayTransport::new(endpoint.clone())
        .map(gateway_computer_control::GatewayComputerControlClient::new);
    Task::perform(async move {
        let client = client?;
        let status = client.computer_control_status().await.map_err(safe_error)?;
        let approvals = client.computer_control_approvals().await.map_err(safe_error)?;
        Ok((status, approvals))
    }, move |result| message(Event::Loaded { identity: identity.clone(), generation, result }))
}

pub fn update(state: &mut ComputerControlSettingsState, endpoint: &GatewayEndpoint, session: Option<&str>, workspace: Option<&str>, event: Event) -> (Task<Event>, Option<Notice>) {
    let mut notice = None;
    let task = update_inner(state, endpoint, session, workspace, event, &mut notice);
    (task, notice)
}

#[derive(Debug, Clone)]
pub enum Notice { Error(String), Success(String) }
fn notify(notice: &mut Option<Notice>, value: Notice) -> Task<Event> { *notice = Some(value); Task::none() }
fn update_inner(state: &mut ComputerControlSettingsState, endpoint: &GatewayEndpoint, session: Option<&str>, workspace: Option<&str>, event: Event, notice: &mut Option<Notice>) -> Task<Event> {
    let identity = ControlIdentity::from_endpoint(endpoint);
    if state.identity.as_ref() != Some(&identity) {
        let generation = state.generation + 1;
        let pairing_generation = state.pairing_generation + 1;
        *state = Default::default();
        state.generation = generation;
        state.pairing_generation = pairing_generation;
        state.identity = Some(identity.clone());
    }
    match event {
        Event::Refresh => refresh(state, endpoint),
        Event::Loaded { identity: source, generation, result } => {
            if source != identity { return Task::none(); }
            if !state.apply_loaded(generation, result) { return Task::none(); }
            if let Some(error) = state.error.clone() { return notify(notice, Notice::Error(error.into())); }
            Task::none()
        }
        Event::PairOpen(feature @ (ControlSetting::Excel | ControlSetting::Word)) => {
            if state.busy() || !state.feature(feature).is_some_and(|s| s.enabled) { return Task::none(); }
            let Some(session) = session.map(str::to_owned) else {
                return notify(notice, Notice::Error("请先打开要授权的聊天会话".into()));
            };
            state.pairing_generation += 1;
            state.pairing = Some(OfficePairingDraft { feature, document: String::new(), session,
                workspace: workspace.map(std::path::PathBuf::from), credential: None });
            Task::none()
        }
        Event::PairOpen(_) => Task::none(),
        Event::PairDocument(value) => {
            if !state.busy() {
                if let Some(draft) = &mut state.pairing { if draft.credential.is_none() { draft.document = value; } }
            }
            Task::none()
        }
        Event::CopyCredential => {
            let Some(credential) = state.pairing.as_ref()
                .and_then(|draft| draft.credential.as_ref()) else { return Task::none(); };
            iced::clipboard::write(credential.0.clone())
        }
        Event::PairClose => {
            // Keep the modal until an in-flight credential has arrived, so it can be consumed.
            if !state.saving { state.pairing = None; }
            Task::none()
        }
        Event::CredentialExpired { identity: source, generation } => {
            if source == identity && state.pairing_generation == generation {
                state.pairing = None;
            }
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
                Err(error) => notify(notice, Notice::Error(error.into())),
            }
        }
        Event::Completed { identity: source, generation, result } => {
            if source != identity || generation != state.generation || !state.saving { return Task::none(); }
            state.saving = false;
            let mut expiry = false;
            let toast = match result {
                Ok(credential) => {
                    if let Some(draft) = &mut state.pairing { draft.credential = credential; expiry = draft.credential.is_some(); }
                    notify(notice, Notice::Success("服务端已确认授权更改".into()))
                }
                Err(error) => notify(notice, Notice::Error(error.into())),
            };
            let reload = refresh(state, endpoint);
            let expiry_task = if expiry {
                let generation = state.pairing_generation;
                Task::perform(async { tokio::time::sleep(std::time::Duration::from_secs(60)).await },
                    move |_| message(Event::CredentialExpired { identity: identity.clone(), generation }))
            } else { Task::none() };
            Task::batch([toast, reload, expiry_task])
        }
        event @ (Event::Toggle(..) | Event::Revoke(_) | Event::PairConfirm) => {
            if state.busy() { return Task::none(); }
            let Some(status) = &state.status else { return refresh(state, endpoint); };
            let version = status.permission_version;
            let pairing = if matches!(event, Event::PairConfirm) {
                let Some(draft) = state.pairing.clone() else { return Task::none(); };
                if draft.credential.is_some() { return Task::none(); }
                if draft.document.trim().is_empty() || draft.document.len() > 256
                    || session != Some(draft.session.as_str())
                    || workspace.map(std::path::PathBuf::from) != draft.workspace {
                    return notify(notice, Notice::Error("文档身份不能为空，且须保持原聊天和项目上下文".into()));
                }
                Some(draft)
            } else { state.pairing = None; None };
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
                        client.computer_control_save(&ControlSettingsUpdate { permission_version: version, feature, enabled }).await.map_err(safe_error)?;
                        Ok(None)
                    }
                    Event::Revoke(scope) => {
                        client.computer_control_revoke(&ControlGrantRevocation { permission_version: version, scope }).await.map_err(safe_error)?;
                        Ok(None)
                    }
                    Event::PairConfirm => {
                        let draft = pairing.ok_or_else(|| "配对信息已失效".to_owned())?;
                        let response = client.computer_control_pair(&ControlPairingConfirmation {
                            permission_version: version, session_id: ControlSessionId(draft.session), workspace_root: draft.workspace,
                            document_id: ControlDocumentId(draft.document.trim().to_owned()), feature: draft.feature,
                        }).await.map_err(safe_error)?;
                        Ok(Some(PairingCredential(response.credential)))
                    }
                    _ => unreachable!(),
                }
            }, move |result| message(Event::Completed { identity: identity.clone(), generation, result }))
        }
    }
}
