// 验证员工资源草稿、加载反馈与成员索引边界。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use crate::app::message::settings::{
    StaffResourceMessage, StaffTeamDraftField, StaffTeamMemberDraftField, StaffWorkerDraftField,
};
use vw_gateway_client::vw_api_types::agent_scheduler::{
    AgentTeamDto, AgentTeamMemberDto, AgentTeamStatusDto, AgentWorkerDto, AgentWorkerStatusDto,
};
use vw_gateway_client::vw_api_types::common::TimestampMs;

fn app() -> crate::app::App {
    crate::app::App::new().0
}

fn worker(id: &str) -> AgentWorkerDto {
    AgentWorkerDto {
        id: id.to_string(),
        runtime: Some("matrix".to_string()),
        deployment: Some("desktop".to_string()),
        model: "gpt-5".to_string(),
        model_provider: Some("openai".to_string()),
        status: AgentWorkerStatusDto::Idle,
        desired_state: Some("Running".to_string()),
        team_id: Some("platform".to_string()),
        worker_role: Some("implementer".to_string()),
        matrix_user_id: Some(format!("@{id}:example.test")),
        room_id: Some("!worker:example.test".to_string()),
        container_state: None,
        idle_since_ms: None,
        last_seen_at_ms: None,
        last_heartbeat_at: None,
        capabilities: Vec::new(),
        updated_at_ms: TimestampMs(100),
    }
}

fn team(id: &str) -> AgentTeamDto {
    AgentTeamDto {
        id: id.to_string(),
        name: "Platform".to_string(),
        status: AgentTeamStatusDto::Active,
        leader_id: "worker-a".to_string(),
        team_room_id: Some("!team:example.test".to_string()),
        leader_dm_room_id: None,
        leader_ready: true,
        ready_workers: 1,
        total_workers: 1,
        members: vec![AgentTeamMemberDto {
            worker_id: "worker-a".to_string(),
            runtime_name: Some("matrix".to_string()),
            role: "lead".to_string(),
            room_id: None,
            matrix_user_id: None,
            ready: true,
        }],
        updated_at_ms: TimestampMs(200),
    }
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn refresh_without_project_sets_both_resource_errors_and_stops_loading() {
    let mut app = app();
    app.project_path = None;
    app.staff_settings.workers.loading = true;
    app.staff_settings.workers.success = Some("stale worker success".to_string());
    app.staff_settings.teams.loading = true;
    app.staff_settings.teams.success = Some("stale team success".to_string());

    let _task = super::update(&mut app, StaffResourceMessage::Refresh);

    assert!(!app.staff_settings.workers.loading);
    assert!(
        app.staff_settings.workers.error.as_deref().is_some_and(|error| error.contains("project"))
    );
    assert_eq!(app.staff_settings.workers.success, None);
    assert!(!app.staff_settings.teams.loading);
    assert!(
        app.staff_settings.teams.error.as_deref().is_some_and(|error| error.contains("project"))
    );
    assert_eq!(app.staff_settings.teams.success, None);
}

#[test]
fn worker_draft_messages_update_all_fields_and_clear_feedback() {
    let mut app = app();
    let _task = super::update(&mut app, StaffResourceMessage::WorkerNewRequested);
    app.staff_settings.workers.error = Some("stale error".to_string());
    app.staff_settings.workers.success = Some("stale success".to_string());

    for (field, value) in [
        (StaffWorkerDraftField::WorkerId, "worker-edited"),
        (StaffWorkerDraftField::Runtime, "matrix"),
        (StaffWorkerDraftField::Deployment, "desktop"),
        (StaffWorkerDraftField::Model, "gpt-5"),
        (StaffWorkerDraftField::ModelProvider, "openai"),
        (StaffWorkerDraftField::TeamId, "platform"),
        (StaffWorkerDraftField::WorkerRole, "reviewer"),
        (StaffWorkerDraftField::MatrixUserId, "@worker:example.test"),
        (StaffWorkerDraftField::RoomId, "!worker:example.test"),
    ] {
        let _task = super::update(
            &mut app,
            StaffResourceMessage::WorkerFieldChanged(field, value.to_string()),
        );
    }
    let _task =
        super::update(&mut app, StaffResourceMessage::WorkerCapabilitiesChanged(Vec::new()));

    let draft = app.staff_settings.workers.draft.as_ref().expect("worker draft");
    assert_eq!(draft.worker_id, "worker-edited");
    assert_eq!(draft.runtime, "matrix");
    assert_eq!(draft.deployment, "desktop");
    assert_eq!(draft.model, "gpt-5");
    assert_eq!(draft.model_provider, "openai");
    assert_eq!(draft.team_id, "platform");
    assert_eq!(draft.worker_role, "reviewer");
    assert_eq!(draft.matrix_user_id, "@worker:example.test");
    assert_eq!(draft.room_id, "!worker:example.test");
    assert!(draft.capabilities.is_empty());
    assert_eq!(app.staff_settings.workers.error, None);
    assert_eq!(app.staff_settings.workers.success, None);
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn team_draft_messages_update_members_and_ignore_out_of_bounds_indices() {
    let mut app = app();
    let _task = super::update(&mut app, StaffResourceMessage::TeamNewRequested);

    for (field, value) in [
        (StaffTeamDraftField::TeamId, "team-edited"),
        (StaffTeamDraftField::Name, "Edited Team"),
        (StaffTeamDraftField::LeaderId, "worker-a"),
        (StaffTeamDraftField::TeamRoomId, "!team:example.test"),
        (StaffTeamDraftField::LeaderDmRoomId, "!leader:example.test"),
    ] {
        let _task = super::update(
            &mut app,
            StaffResourceMessage::TeamFieldChanged(field, value.to_string()),
        );
    }
    let _task = super::update(
        &mut app,
        StaffResourceMessage::TeamStatusChanged(AgentTeamStatusDto::Active),
    );
    let _task =
        super::update(&mut app, StaffResourceMessage::TeamMemberAdded("worker-a".to_string()));
    for (field, value) in [
        (StaffTeamMemberDraftField::WorkerId, "worker-edited"),
        (StaffTeamMemberDraftField::RuntimeName, "matrix"),
        (StaffTeamMemberDraftField::Role, "lead"),
        (StaffTeamMemberDraftField::RoomId, "!worker:example.test"),
        (StaffTeamMemberDraftField::MatrixUserId, "@worker:example.test"),
    ] {
        let _task = super::update(
            &mut app,
            StaffResourceMessage::TeamMemberFieldChanged(0, field, value.to_string()),
        );
    }
    let _task = super::update(&mut app, StaffResourceMessage::TeamMemberReadyToggled(0, true));
    let _task = super::update(&mut app, StaffResourceMessage::TeamMemberRemoved(7));

    let draft = app.staff_settings.teams.draft.as_ref().expect("team draft");
    assert_eq!(draft.team_id, "team-edited");
    assert_eq!(draft.name, "Edited Team");
    assert_eq!(draft.status, AgentTeamStatusDto::Active);
    assert_eq!(draft.leader_id, "worker-a");
    assert_eq!(draft.team_room_id, "!team:example.test");
    assert_eq!(draft.leader_dm_room_id, "!leader:example.test");
    assert_eq!(draft.members.len(), 1);
    let member = &draft.members[0];
    assert_eq!(member.worker_id, "worker-edited");
    assert_eq!(member.runtime_name, "matrix");
    assert_eq!(member.role, "lead");
    assert_eq!(member.room_id, "!worker:example.test");
    assert_eq!(member.matrix_user_id, "@worker:example.test");
    assert!(member.ready);

    let _task = super::update(&mut app, StaffResourceMessage::TeamMemberRemoved(0));
    assert!(app.staff_settings.teams.draft.as_ref().expect("team draft").members.is_empty());
}

#[test]
fn load_failures_clear_loading_and_stale_success_messages() {
    let mut app = app();
    app.staff_settings.workers.loading = true;
    app.staff_settings.workers.success = Some("stale worker success".to_string());
    app.staff_settings.teams.loading = true;
    app.staff_settings.teams.success = Some("stale team success".to_string());

    let _task = super::update(
        &mut app,
        StaffResourceMessage::WorkersLoaded(Err("worker failure".to_string())),
    );
    let _task =
        super::update(&mut app, StaffResourceMessage::TeamsLoaded(Err("team failure".to_string())));

    assert!(!app.staff_settings.workers.loading);
    assert!(app.staff_settings.workers.error.is_some());
    assert_eq!(app.staff_settings.workers.success, None);
    assert!(!app.staff_settings.teams.loading);
    assert!(app.staff_settings.teams.error.is_some());
    assert_eq!(app.staff_settings.teams.success, None);
}

#[test]
fn resource_loaded_success_replaces_workers_and_teams() {
    let mut app = app();
    app.staff_settings.workers.loading = true;
    app.staff_settings.teams.loading = true;

    let _task = super::update(
        &mut app,
        StaffResourceMessage::WorkersLoaded(Ok(vec![worker("worker-a"), worker("worker-b")])),
    );
    let _task =
        super::update(&mut app, StaffResourceMessage::TeamsLoaded(Ok(vec![team("platform")])));

    assert!(!app.staff_settings.workers.loading);
    assert_eq!(app.staff_settings.workers.workers.len(), 2);
    assert_eq!(app.staff_settings.workers.workers[0].id, "worker-a");
    assert!(!app.staff_settings.teams.loading);
    assert_eq!(app.staff_settings.teams.teams.len(), 1);
    assert_eq!(app.staff_settings.teams.teams[0].id, "platform");
}

// 提交与删除：覆盖保存、安装或移除操作前后的状态衔接。
// 这里关注校验、任务载荷和结果反馈，不绑定底层实现。
#[test]
fn resource_saved_success_clears_saving_and_applies_saved_items() {
    let mut app = app();
    app.staff_settings.workers.saving = true;
    app.staff_settings.teams.saving = true;

    let _task =
        super::update(&mut app, StaffResourceMessage::WorkerSaved(Ok(worker("worker-saved"))));
    let _task = super::update(&mut app, StaffResourceMessage::TeamSaved(Ok(team("team-saved"))));

    assert!(!app.staff_settings.workers.saving);
    assert_eq!(app.staff_settings.workers.workers[0].id, "worker-saved");
    assert!(!app.staff_settings.teams.saving);
    assert_eq!(app.staff_settings.teams.teams[0].id, "team-saved");
}

#[test]
fn save_requests_short_circuit_while_already_saving() {
    let mut app = app();
    app.project_path = Some("/tmp/project".to_string());
    app.staff_settings.workers.saving = true;
    app.staff_settings.teams.saving = true;

    let _worker_task = super::update(&mut app, StaffResourceMessage::WorkerSaveRequested);
    let _team_task = super::update(&mut app, StaffResourceMessage::TeamSaveRequested);

    assert!(app.staff_settings.workers.saving);
    assert!(app.staff_settings.workers.error.is_none());
    assert!(app.staff_settings.teams.saving);
    assert!(app.staff_settings.teams.error.is_none());
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn save_requests_without_project_fail_before_gateway_work() {
    let mut app = app();
    app.project_path = None;

    let _worker_task = super::update(&mut app, StaffResourceMessage::WorkerSaveRequested);
    let _team_task = super::update(&mut app, StaffResourceMessage::TeamSaveRequested);

    assert!(!app.staff_settings.workers.saving);
    assert!(
        app.staff_settings.workers.error.as_deref().is_some_and(|error| error.contains("project"))
    );
    assert!(!app.staff_settings.teams.saving);
    assert!(
        app.staff_settings.teams.error.as_deref().is_some_and(|error| error.contains("project"))
    );
}
