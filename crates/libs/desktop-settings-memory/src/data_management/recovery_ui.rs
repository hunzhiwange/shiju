//! Recovery is always previewed; only explicit confirmation submits a write.
use super::export_ui::Identity;
use iced::Task;
use gateway_app_data::{DataBackupPreview, PreviewDataBackupRequest, DataRecoveryDiagnosis, DataTargetHealth, DataBackupRestoreStatus, DataRestoreProgress};
use {
    vw_api_transport::ProjectScope,
    vw_api_app_operations::data_management::*,
};

#[derive(Clone, Default)]
pub struct State {
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
pub enum Event {
    Feedback(bool, String),
    Open, Close, Principal(String), Diagnose, Preview(String), Confirm, Refresh,
    Diagnosed(Identity, u64, Result<DataRecoveryDiagnosis, ()>),
    Previewed(Identity, u64, Result<DataBackupPreview, ()>),
    Restored(Identity, u64, Result<DataRestoreProgress, ()>),
}
pub enum Output { Event(Event), Error(String), Success(String) }
fn message(event: Event) -> Output { Output::Event(event) }
fn notify_error(value: impl Into<String>) -> Task<Output> { Task::done(Output::Error(value.into())) }
pub fn visible(state: &State, current: Option<&Identity>) -> bool {
    state.open && current.is_some() && current == state.identity.as_ref()
}
pub fn update(state: &mut State, current: Option<Identity>, client: Option<vw_gateway_client::GatewayClient>, token: String, event: Event) -> Task<Output> {
    let Some(current) = current else { *state = State::default(); return Task::none(); };
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
                Err(()) => { state.error = Some("恢复结果暂不可用。请查询已有任务，勿重复恢复；如无任务 ID，请联系执行主机管理员核对。".into()); return notify_error("恢复结果需核对，请勿重复提交"); }
            }
            return Task::none();
        }
        _ => {}
    }
    if state.busy { return Task::none(); }
    let Ok(client) = client.clone().ok_or(()) else { return Task::none(); };
    let project = current.project.clone();
    let Ok(scope) = ProjectScope::new(client.endpoint().tenant_scope().clone(), project.into()) else { return Task::none(); };
    let target = DataManagementTarget { scope, target_host_id: current.host.clone(), principal_id: "desktop-local-user".into() };
    let token = token.clone();
    let generation = state.generation.wrapping_add(1);
    let task = match event {
        Event::Diagnose if !state.submitted => {
            state.preview = None; state.diagnosis = None;
            Task::perform(async move { message(Event::Diagnosed(current, generation, gateway_app_data::GatewayAppDataClient::new(client.transport().clone()).data_backup_diagnose(&ListDataBackupsRequest { target }, Some(&token)).await.map_err(|_| ()))) }, |m| m)
        }
        Event::Preview(id) if !state.submitted && state.diagnosis.as_ref().is_some_and(|d| d.target_health != DataTargetHealth::VersionUnsupported && d.backups.iter().any(|b| b.id == id && b.restore_status == DataBackupRestoreStatus::RequiresOwnerValidation)) => {
            state.preview = None;
            Task::perform(async move { message(Event::Previewed(current, generation, gateway_app_data::GatewayAppDataClient::new(client.transport().clone()).data_backup_restore_preview(&PreviewDataBackupRequest { target, backup_id: DataBackupId(id) }, Some(&token)).await.map_err(|_| ()))) }, |m| m)
        }
        Event::Confirm if !state.submitted && state.preview.as_ref().is_some_and(|p| p.backup.target == target) => {
            let preview = state.preview.as_ref().unwrap();
            let request = RestoreDataBackupRequest { target, backup_id: preview.backup.backup_id.clone(), expected_revision: preview.backup.revision.clone(), categories: preview.backup.categories.clone(), confirmed: true,
                idempotency_key: format!("desktop-recovery-{}-{generation}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()) };
            state.submitted = true;
            Task::perform(async move { message(Event::Restored(current, generation, gateway_app_data::GatewayAppDataClient::new(client.transport().clone()).data_backup_restore(&request, Some(&token)).await.map_err(|_| ()))) }, |m| m)
        }
        Event::Refresh if state.progress.is_some() => {
            let request = GetDataOperationRequest { target, operation_id: state.progress.as_ref().unwrap().operation_id.clone().into() };
            Task::perform(async move { message(Event::Restored(current, generation, gateway_app_data::GatewayAppDataClient::new(client.transport().clone()).data_backup_restore_operation(&request, Some(&token)).await.map_err(|_| ()))) }, |m| m)
        }
        _ => return Task::none(),
    };
    state.busy = true; state.error = None; state.generation = generation;
    task
}
