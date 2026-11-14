//! Recovery is always previewed; only explicit confirmation submits a write.
use super::export_ui::{identity, Identity};
use crate::app::{App, Message, message::SettingsMessage};
use iced::Task;
use vw_gateway_client::{DataBackupPreview, PreviewDataBackupRequest, DataRecoveryDiagnosis, DataTargetHealth, DataBackupRestoreStatus, DataRestoreProgress};
use vw_gateway_client::vw_api_types::{ProjectScope, agent_apps::data_management::*};

#[derive(Clone, Default)]
pub(crate) struct State {
    pub open: bool,
    pub busy: bool,
    pub principal: String,
    pub diagnosis: Option<DataRecoveryDiagnosis>,
    pub preview: Option<DataBackupPreview>,
    pub progress: Option<DataRestoreProgress>,
    pub submitted: bool,
    pub error: Option<String>,
    identity: Option<Identity>,
    generation: u64,
}
#[derive(Debug, Clone)]
pub(crate) enum Event {
    Open, Close, Principal(String), Diagnose, Preview(String), Confirm, Refresh,
    Diagnosed(Identity, u64, Result<DataRecoveryDiagnosis, ()>),
    Previewed(Identity, u64, Result<DataBackupPreview, ()>),
    Restored(Identity, u64, Result<DataRestoreProgress, ()>),
}
pub(crate) fn message(event: Event) -> Message { Message::Settings(SettingsMessage::DataRecovery(event)) }
pub(crate) fn visible(app: &App) -> bool {
    app.apps_catalog.recovery.open && identity(app).is_some() && identity(app) == app.apps_catalog.recovery.identity
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let Some(current) = identity(app) else { app.apps_catalog.recovery = State::default(); return Task::none(); };
    let state = &mut app.apps_catalog.recovery;
    if matches!(event, Event::Open) {
        if state.open && state.identity.as_ref() == Some(&current) { return Task::none(); }
        *state = State { open: true, identity: Some(current), generation: state.generation.wrapping_add(1), ..Default::default() };
        return Task::none();
    }
    if !state.open || state.identity.as_ref() != Some(&current) { return Task::none(); }
    match event {
        Event::Close if !state.busy => { state.open = false; return Task::none(); }
        Event::Principal(value) if !state.busy && !state.submitted => {
            state.principal = value; state.preview = None; state.diagnosis = None; return Task::none();
        }
        Event::Diagnosed(id, generation, result) if id == current && generation == state.generation && state.busy => {
            state.busy = false;
            match result { Ok(value) => state.diagnosis = Some(value), Err(()) => state.error = Some("诊断失败，请检查授权与执行主机连接；原数据未修改。".into()) }
            return Task::none();
        }
        Event::Previewed(id, generation, result) if id == current && generation == state.generation && state.busy => {
            state.busy = false;
            match result { Ok(value) => state.preview = Some(value), Err(()) => state.error = Some("备份预览失败，未提交恢复。".into()) }
            return Task::none();
        }
        Event::Restored(id, generation, result) if id == current && generation == state.generation && state.busy => {
            state.busy = false;
            match result {
                Ok(value) => state.progress = Some(value),
                Err(()) => { state.error = Some("恢复结果暂不可用。请查询已有任务，勿重复恢复；如无任务 ID，请联系执行主机管理员核对。".into()); return app.show_error_toast("恢复结果需核对，请勿重复提交"); }
            }
            return Task::none();
        }
        _ => {}
    }
    if state.busy || state.principal.trim().is_empty() { return Task::none(); }
    let Ok(client) = crate::app::config::gateway_client() else { return Task::none(); };
    let Some(project) = app.project_id.clone() else { return Task::none(); };
    let Ok(scope) = ProjectScope::new(client.endpoint().tenant_scope().clone(), project.into()) else { return Task::none(); };
    let target = DataManagementTarget { scope, target_host_id: app.apps_catalog.host.clone(), principal_id: state.principal.trim().into() };
    let token = app.apps_catalog.token.clone();
    let generation = state.generation.wrapping_add(1);
    let task = match event {
        Event::Diagnose if !state.submitted => {
            state.preview = None; state.diagnosis = None;
            Task::perform(async move { message(Event::Diagnosed(current, generation, client.data_backup_diagnose(&ListDataBackupsRequest { target }, Some(&token)).await.map_err(|_| ()))) }, |m| m)
        }
        Event::Preview(id) if !state.submitted && state.diagnosis.as_ref().is_some_and(|d| d.target_health != DataTargetHealth::VersionUnsupported && d.backups.iter().any(|b| b.id == id && b.restore_status == DataBackupRestoreStatus::RequiresOwnerValidation)) => {
            state.preview = None;
            Task::perform(async move { message(Event::Previewed(current, generation, client.data_backup_restore_preview(&PreviewDataBackupRequest { target, backup_id: DataBackupId(id) }, Some(&token)).await.map_err(|_| ()))) }, |m| m)
        }
        Event::Confirm if !state.submitted && state.preview.as_ref().is_some_and(|p| p.backup.target == target) => {
            let preview = state.preview.as_ref().unwrap();
            let request = RestoreDataBackupRequest { target, backup_id: preview.backup.backup_id.clone(), expected_revision: preview.backup.revision.clone(), categories: preview.backup.categories.clone(), confirmed: true,
                idempotency_key: format!("desktop-recovery-{}-{generation}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()) };
            state.submitted = true;
            Task::perform(async move { message(Event::Restored(current, generation, client.data_backup_restore(&request, Some(&token)).await.map_err(|_| ()))) }, |m| m)
        }
        Event::Refresh if state.progress.is_some() => {
            let request = GetDataOperationRequest { target, operation_id: state.progress.as_ref().unwrap().operation_id.clone().into() };
            Task::perform(async move { message(Event::Restored(current, generation, client.data_backup_restore_operation(&request, Some(&token)).await.map_err(|_| ()))) }, |m| m)
        }
        _ => return Task::none(),
    };
    state.busy = true; state.error = None; state.generation = generation;
    task
}
