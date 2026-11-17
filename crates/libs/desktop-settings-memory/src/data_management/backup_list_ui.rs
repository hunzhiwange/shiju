//! Executor-owned backup maintenance; stale responses never cross catalog identities.
use super::export_ui::Identity;
use iced::Task;
use gateway_app_data::{DataBackupPolicy, DataBackupLocation, DataBackupLocationDeletion, DeleteDataBackupLocationRequest};
use {
    vw_api_transport::ProjectScope,
    vw_api_app_operations::data_management::*,
};

#[derive(Clone, Debug)]
pub enum Event {
    Feedback(bool, String),
    Open, Close, Principal(String), Name(String), Interval(String), Retain(String),
    Reveal(String), Revealed(Identity, u64, Result<String, String>),
    Cleanup(String), Cleaned(Identity, u64, Result<DataBackupLocationDeletion, String>),
    Refresh, Create, Rename(DataBackupSummary), Delete(DataBackupSummary), Cancel, Confirm, SavePolicy,
    Loaded(Identity, u64, Result<Snapshot, String>),
    Changed(Identity, u64, Result<(), String>),
}
#[derive(Clone, Debug)]
pub struct Snapshot {
    rows: Vec<DataBackupSummary>,
    policy: DataBackupPolicy,
    locations: Vec<DataBackupLocation>,
}
#[derive(Clone, Debug)]
pub enum Pending { Cleanup(String), Rename(DataBackupSummary), Delete(DataBackupSummary) }
#[derive(Clone, Default)]
pub struct State {
    pub open: bool,
    pub busy: bool,
    pub principal: String,
    pub name: String,
    pub interval: String,
    pub retain: String,
    pub rows: Vec<DataBackupSummary>,
    pub locations: Vec<DataBackupLocation>,
    pub pending: Option<Pending>,
    pub cleanup_result: Option<String>,
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
        if state.open && state.identity.as_ref() == Some(&current) { return Task::none(); }
        *state = State { open: true, identity: Some(current), generation: state.generation.wrapping_add(1), ..Default::default() };
        return Task::none();
    }
    if !state.open || state.identity.as_ref() != Some(&current) { return Task::none(); }
    match event {
        Event::Revealed(request, generation, result) if request == current && generation == state.generation && state.busy => {
            // Recheck identity before any local filesystem or system action.
            state.busy = false;
            let Ok(client) = client.clone().ok_or(()) else { return Task::none(); };
            if !crate::system_settings_data::locations::can_reveal(client.endpoint().is_loopback(), &current.host) { return notify_error("远程目录不可在本机定位"); }
            return match result {
                Ok(path) => {
                    state.busy = true;
                    Task::perform(async move {
                        desktop_config::config_file_manager::reveal_gateway_path_locally(client.endpoint(), &path).await
                    }, move |result| message(Event::Changed(current.clone(), generation, result)))
                }
                Err(error) => notify_error(error),
            };
        }
        Event::Reveal(id) => {
            let Ok(client) = client.clone().ok_or(()) else { return Task::none(); };
            if !crate::system_settings_data::locations::can_reveal(client.endpoint().is_loopback(), &current.host) { return notify_error("远程目录不可在本机定位"); }
            if state.busy || !state.locations.iter().any(|location| location.id == id) { return Task::none(); }
            let Ok(scope) = ProjectScope::new(current.context.tenant.clone(), current.project.clone().into()) else { return Task::none(); };
            let target = DataManagementTarget { scope, target_host_id: current.host.clone(), principal_id: "desktop-local-user".into() };
            state.busy = true;
            state.generation = state.generation.wrapping_add(1);
            let generation = state.generation;
            let token = token.clone();
            return Task::perform(async move {
                let result = gateway_app_data::GatewayAppDataClient::new(client.transport().clone()).data_backup_location_path(&target, &id, Some(&token)).await.map_err(|e| e.to_string());
                message(Event::Revealed(current, generation, result))
            }, |message| message);
        }
        Event::Cleaned(request, generation, result) if request == current && generation == state.generation && state.busy => {
            state.busy = false;
            state.pending = None;
            return match result {
                Ok(report) => {
                    state.cleanup_result = Some(crate::system_settings_data::locations::cleanup_summary(&report));
                    update(state, Some(current), client, token, Event::Refresh)
                }
                Err(error) => notify_error(format!("清理结果可能不完整，请刷新分类核对：{error}")),
            };
        }
        Event::Loaded(request, generation, result) if request == current && generation == state.generation && state.busy => {
            state.busy = false;
            return match result {
                Ok(snapshot) => {
                    state.rows = snapshot.rows;
                    state.locations = snapshot.locations;
                    state.interval = snapshot.policy.interval_hours.to_string();
                    state.retain = snapshot.policy.retain_count.to_string();
                    Task::none()
                }
                Err(error) => notify_error(error),
            };
        }
        Event::Changed(request, generation, result) if request == current && generation == state.generation && state.busy => {
            state.busy = false;
            state.pending = None;
            return match result {
                Ok(()) => Task::batch([success("备份维护已完成"), update(state, Some(current), client, token, Event::Refresh)]),
                Err(error) => notify_error(error),
            };
        }
        _ if state.busy => return Task::none(),
        Event::Close => state.open = false,
        Event::Principal(value) => {
            state.principal = value; state.cleanup_result = None;
            state.rows.clear(); state.locations.clear(); state.pending = None;
            state.interval.clear(); state.retain.clear();
        }
        Event::Name(value) => state.name = value,
        Event::Interval(value) => state.interval = value,
        Event::Retain(value) => state.retain = value,
        Event::Rename(row) => { state.name = row.name.clone(); state.pending = Some(Pending::Rename(row)); }
        Event::Delete(row) => state.pending = Some(Pending::Delete(row)),
        Event::Cleanup(id) => {
            if state.locations.iter().any(|location| location.id == id && crate::system_settings_data::locations::can_cleanup(location)) {
                state.pending = Some(Pending::Cleanup(id));
            }
        }
        Event::Cancel => state.pending = None,
        action @ (Event::Refresh | Event::Create | Event::Confirm | Event::SavePolicy) => {

            let Ok(client) = client.clone().ok_or(()) else { return Task::none(); };
            let Ok(scope) = ProjectScope::new(current.context.tenant.clone(), current.project.clone().into()) else { return Task::none(); };
            let target = DataManagementTarget { scope, target_host_id: current.host.clone(), principal_id: "desktop-local-user".into() };
            let policy = if matches!(action, Event::SavePolicy) {
                let (Ok(interval_hours), Ok(retain_count)) = (state.interval.parse::<u32>(), state.retain.parse::<u32>()) else {
                    return notify_error("周期与保留数必须为非负整数");
                };
                Some(DataBackupPolicy { interval_hours, retain_count })
            } else { None };
            if matches!(action, Event::Create) && state.name.trim().is_empty() { return notify_error("请填写备份名称"); }
            if matches!(action, Event::Confirm) && (state.pending.is_none() || (matches!(state.pending, Some(Pending::Rename(_))) && state.name.trim().is_empty())) { return Task::none(); }
            if let Some(Pending::Cleanup(id)) = &state.pending {
                if matches!(action, Event::Confirm) && !state.locations.iter().any(|location| location.id == *id && crate::system_settings_data::locations::can_cleanup(location)) { return Task::none(); }
            }
            let pending = state.pending.clone();
            let name = state.name.trim().to_owned();
            state.busy = true;
            state.generation = state.generation.wrapping_add(1);
            let generation = state.generation;
            let token = token.clone();
            return Task::perform(async move {
                if matches!(action, Event::Confirm) {
                    if let Some(Pending::Cleanup(id)) = &pending {
                        let result = gateway_app_data::GatewayAppDataClient::new(client.transport().clone()).data_backup_delete_location(&DeleteDataBackupLocationRequest {
                            target, category_id: id.clone(), confirmed: true,
                        }, Some(&token)).await.map_err(|e| e.to_string());
                        return message(Event::Cleaned(current, generation, result));
                    }
                }
                if matches!(action, Event::Refresh) {
                    let result = async {
                        let request = ListDataBackupsRequest { target: target.clone() };
                        let rows = gateway_app_data::GatewayAppDataClient::new(client.transport().clone()).data_backup_list(&request, Some(&token)).await.map_err(|e| e.to_string())?;
                        let policy = gateway_app_data::GatewayAppDataClient::new(client.transport().clone()).data_backup_policy(&target, Some(&token)).await.map_err(|e| e.to_string())?;
                        let locations = gateway_app_data::GatewayAppDataClient::new(client.transport().clone()).data_backup_locations(&request, Some(&token)).await.map_err(|e| e.to_string())?;
                        Ok(Snapshot { rows, policy, locations })
                    }.await;
                    return message(Event::Loaded(current, generation, result));
                }
                let result = async {
                    match action {
                        Event::Create => {
                            let request = CreateDataBackupRequest { target, name, kind: DataArtifactKind::LocalRollback,
                                categories: vec![DataCategory::Providers, DataCategory::Mcp, DataCategory::Skills, DataCategory::Rules, DataCategory::Profiles, DataCategory::Preferences, DataCategory::Usage],
                                idempotency_key: format!("desktop-backup-{}-{generation}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()) };
                            if gateway_app_data::GatewayAppDataClient::new(client.transport().clone()).data_backup_create(&request, Some(&token)).await.map_err(|e| e.to_string())?.is_none() { return Err("未创建备份；请刷新列表核对".into()); }
                        }
                        Event::Confirm => match pending {
                            Some(Pending::Rename(row)) => { gateway_app_data::GatewayAppDataClient::new(client.transport().clone()).data_backup_rename(&RenameDataBackupRequest { target, backup_id: row.backup_id, expected_revision: row.revision, name }, Some(&token)).await.map_err(|e| e.to_string())?; }
                            Some(Pending::Delete(row)) => { gateway_app_data::GatewayAppDataClient::new(client.transport().clone()).data_backup_delete(&DeleteDataBackupRequest { target, backup_id: row.backup_id, expected_revision: row.revision, confirmed: true }, Some(&token)).await.map_err(|e| format!("删除失败（可能存在保护或版本冲突）；请刷新后核对：{e}"))?; }
                            Some(Pending::Cleanup(_)) => return Err("分类清理必须走独立确认路径".into()),
                            None => return Err("没有待确认操作".into()),
                        },
                        Event::SavePolicy => { gateway_app_data::GatewayAppDataClient::new(client.transport().clone()).data_backup_set_policy(&target, &policy.expect("validated policy"), Some(&token)).await.map_err(|e| e.to_string())?; }
                        _ => return Err("不支持的备份操作".into()),
                    }
                    Ok(())
                }.await;
                message(Event::Changed(current, generation, result))
            }, |message| message);
        }
        _ => {}
    }
    Task::none()
}
