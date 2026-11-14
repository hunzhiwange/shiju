//! Authenticated desktop management, with serialized edits and stale-response rejection.
use crate::app::{App, Message};
use crate::app::message::SettingsMessage;
use crate::app::state::ToastKind;
use crate::app::state::{ControlIdentity, OfficePairingDraft, PairingCredential};
use iced::Task;
use vw_gateway_client::vw_api_types::computer_control::*;

#[derive(Debug, Clone)]
pub(crate) enum Event {
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
pub(crate) fn message(event: Event) -> Message { Message::Settings(SettingsMessage::ComputerControl(event)) }

fn safe_error(error: impl std::fmt::Display) -> String {
    // The client only exposes status/transport/decode errors, never response bodies.
    format!("电脑操控请求失败：{error}。请刷新后重试。")
}
fn refresh(app: &mut App) -> Task<Message> {
    let state = &mut app.computer_control_settings;
    if state.busy() { return Task::none(); }
    state.generation += 1;
    let generation = state.generation;
    state.loading = true;
    state.status = None;
    state.approvals.clear();
    state.error = None;
    let identity = ControlIdentity::current();
    state.identity = Some(identity.clone());
    let client = crate::app::config::gateway_client();
    Task::perform(async move {
        let client = client?;
        let status = client.computer_control_status().await.map_err(safe_error)?;
        let approvals = client.computer_control_approvals().await.map_err(safe_error)?;
        Ok((status, approvals))
    }, move |result| message(Event::Loaded { identity: identity.clone(), generation, result }))
}

pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let identity = ControlIdentity::current();
    if app.computer_control_settings.identity.as_ref() != Some(&identity) {
        let generation = app.computer_control_settings.generation + 1;
        let pairing_generation = app.computer_control_settings.pairing_generation + 1;
        app.computer_control_settings = Default::default();
        app.computer_control_settings.generation = generation;
        app.computer_control_settings.pairing_generation = pairing_generation;
        app.computer_control_settings.identity = Some(identity.clone());
    }
    match event {
        Event::Refresh => refresh(app),
        Event::Loaded { identity: source, generation, result } => {
            if source != identity { return Task::none(); }
            let state = &mut app.computer_control_settings;
            if !state.apply_loaded(generation, result) { return Task::none(); }
            if let Some(error) = state.error.clone() { return app.show_toast(ToastKind::Error, error); }
            Task::none()
        }
        Event::PairOpen(feature @ (ControlSetting::Excel | ControlSetting::Word)) => {
            let state = &mut app.computer_control_settings;
            if state.busy() || !state.feature(feature).is_some_and(|s| s.enabled) { return Task::none(); }
            let Some(session) = app.active_session_id.clone() else {
                return app.show_toast(ToastKind::Error, "请先打开要授权的聊天会话");
            };
            state.pairing_generation += 1;
            state.pairing = Some(OfficePairingDraft { feature, document: String::new(), session,
                workspace: app.project_path.as_ref().map(std::path::PathBuf::from), credential: None });
            Task::none()
        }
        Event::PairOpen(_) => Task::none(),
        Event::PairDocument(value) => {
            let state = &mut app.computer_control_settings;
            if !state.busy() {
                if let Some(draft) = &mut state.pairing { if draft.credential.is_none() { draft.document = value; } }
            }
            Task::none()
        }
        Event::CopyCredential => {
            let Some(credential) = app.computer_control_settings.pairing.as_ref()
                .and_then(|draft| draft.credential.as_ref()) else { return Task::none(); };
            iced::clipboard::write(credential.0.clone())
        }
        Event::PairClose => {
            // Keep the modal until an in-flight credential has arrived, so it can be consumed.
            if !app.computer_control_settings.saving { app.computer_control_settings.pairing = None; }
            Task::none()
        }
        Event::CredentialExpired { identity: source, generation } => {
            if source == identity && app.computer_control_settings.pairing_generation == generation {
                app.computer_control_settings.pairing = None;
            }
            Task::none()
        }
        Event::Stop => {
            if app.computer_control_settings.stopping { return Task::none(); }
            app.computer_control_settings.stopping = true;
            let client = crate::app::config::gateway_client();
            Task::perform(async move { client?.computer_control_stop().await.map_err(safe_error) },
                move |result| message(Event::Stopped { identity: identity.clone(), result }))
        }
        Event::Stopped { identity: source, result } => {
            if source != identity { return Task::none(); }
            app.computer_control_settings.stopping = false;
            match result {
                Ok(()) => app.show_toast(ToastKind::Success, "服务端已停止电脑操控"),
                Err(error) => app.show_toast(ToastKind::Error, error),
            }
        }
        Event::Completed { identity: source, generation, result } => {
            let state = &mut app.computer_control_settings;
            if source != identity || generation != state.generation || !state.saving { return Task::none(); }
            state.saving = false;
            let mut expiry = false;
            let toast = match result {
                Ok(credential) => {
                    if let Some(draft) = &mut state.pairing { draft.credential = credential; expiry = draft.credential.is_some(); }
                    app.show_toast(ToastKind::Success, "服务端已确认授权更改")
                }
                Err(error) => app.show_toast(ToastKind::Error, error),
            };
            let reload = refresh(app);
            let expiry_task = if expiry {
                let generation = app.computer_control_settings.pairing_generation;
                Task::perform(async { tokio::time::sleep(std::time::Duration::from_secs(60)).await },
                    move |_| message(Event::CredentialExpired { identity: identity.clone(), generation }))
            } else { Task::none() };
            Task::batch([toast, reload, expiry_task])
        }
        event @ (Event::Toggle(..) | Event::Revoke(_) | Event::PairConfirm) => {
            let state = &mut app.computer_control_settings;
            if state.busy() { return Task::none(); }
            let Some(status) = &state.status else { return refresh(app); };
            let version = status.permission_version;
            let pairing = if matches!(event, Event::PairConfirm) {
                let Some(draft) = state.pairing.clone() else { return Task::none(); };
                if draft.credential.is_some() { return Task::none(); }
                if draft.document.trim().is_empty() || draft.document.len() > 256
                    || app.active_session_id.as_deref() != Some(draft.session.as_str())
                    || app.project_path.as_ref().map(std::path::PathBuf::from) != draft.workspace {
                    return app.show_toast(ToastKind::Error, "文档身份不能为空，且须保持原聊天和项目上下文");
                }
                Some(draft)
            } else { state.pairing = None; None };
            state.generation += 1;
            let generation = state.generation;
            state.saving = true;
            state.error = None;
            let client = crate::app::config::gateway_client();
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
