//! 员工资源编辑状态测试，覆盖成员与团队草稿的选择、替换和请求构建。
//! 必填项错误按字段返回，切换资源时不得让未保存草稿跨成员相互污染。

use super::{
    StaffTeamMemberDraft, StaffTeamResourcesState, StaffWorkerResourcesState, next_resource_id,
    optional_trimmed, required_trimmed, resource_project_path,
};
use serde_json::Value;
use vw_gateway_client::vw_api_types::agent_scheduler::{
    AgentCapabilityDto, AgentCapabilityKindDto, AgentTeamDto, AgentTeamMemberDto,
    AgentTeamStatusDto, AgentWorkerDto, AgentWorkerStatusDto,
};
use vw_gateway_client::vw_api_types::common::TimestampMs;

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
        container_state: Some("running".to_string()),
        idle_since_ms: Some(TimestampMs(101)),
        last_seen_at_ms: Some(TimestampMs(102)),
        last_heartbeat_at: Some("2026-07-14T08:00:00Z".to_string()),
        capabilities: vec![AgentCapabilityDto {
            kind: AgentCapabilityKindDto::Runtime,
            name: "matrix".to_string(),
            metadata: Value::Null,
        }],
        updated_at_ms: TimestampMs(103),
    }
}

fn team() -> AgentTeamDto {
    AgentTeamDto {
        id: "platform".to_string(),
        name: "Platform".to_string(),
        status: AgentTeamStatusDto::Active,
        leader_id: "worker-a".to_string(),
        team_room_id: Some("!team:example.test".to_string()),
        leader_dm_room_id: Some("!leader:example.test".to_string()),
        leader_ready: true,
        ready_workers: 1,
        total_workers: 2,
        members: vec![
            AgentTeamMemberDto {
                worker_id: "worker-a".to_string(),
                runtime_name: Some("matrix".to_string()),
                role: "lead".to_string(),
                room_id: Some("!worker-a:example.test".to_string()),
                matrix_user_id: Some("@worker-a:example.test".to_string()),
                ready: true,
            },
            AgentTeamMemberDto {
                worker_id: "worker-b".to_string(),
                runtime_name: Some("local".to_string()),
                role: "reviewer".to_string(),
                room_id: None,
                matrix_user_id: None,
                ready: false,
            },
        ],
        updated_at_ms: TimestampMs(201),
    }
}

#[test]
fn staff_resource_state_builds_independent_worker_and_team_requests() {
    let workers = vec![worker("worker-a"), worker("worker-b")];
    let mut worker_state = StaffWorkerResourcesState::default();
    let mut team_state = StaffTeamResourcesState::default();

    worker_state.replace_workers(workers.clone());
    team_state.replace_teams(vec![team()]);
    worker_state.select("worker-a");
    team_state.select("platform");

    worker_state.draft.as_mut().expect("worker draft").model = "gpt-5.2".to_string();
    team_state.draft.as_mut().expect("team draft").name = "Core Platform".to_string();

    let worker_request = worker_state.request().expect("valid worker request");
    assert_eq!(worker_request.worker_id, "worker-a");
    assert_eq!(worker_request.model.as_deref(), Some("gpt-5.2"));
    assert_eq!(worker_request.status, Some(AgentWorkerStatusDto::Idle));
    assert_eq!(worker_request.last_heartbeat_at.as_deref(), Some("2026-07-14T08:00:00Z"));

    let team_request = team_state.request(&workers).expect("valid team request");
    assert_eq!(team_request.team_id, "platform");
    assert_eq!(team_request.name, "Core Platform");
    assert_eq!(team_request.ready_workers, 1);
    assert_eq!(team_request.total_workers, 2);
    assert!(team_request.leader_ready);

    assert_eq!(worker_state.draft.as_ref().expect("worker draft").model, "gpt-5.2");
    assert_eq!(team_state.draft.as_ref().expect("team draft").name, "Core Platform");

    let duplicate = team_state.draft.as_mut().expect("team draft").members[1].clone();
    team_state
        .draft
        .as_mut()
        .expect("team draft")
        .members
        .push(StaffTeamMemberDraft { worker_id: "worker-a".to_string(), ..duplicate });
    assert!(
        team_state.request(&workers).expect_err("duplicate member rejected").contains("unique")
    );

    team_state.draft.as_mut().expect("team draft").members.pop();
    team_state.draft.as_mut().expect("team draft").leader_id = "missing-worker".to_string();
    assert!(team_state.request(&workers).expect_err("missing leader rejected").contains("leader"));

    team_state.draft.as_mut().expect("team draft").leader_id = "worker-a".to_string();
    team_state.draft.as_mut().expect("team draft").members[1].worker_id =
        "unknown-worker".to_string();
    assert!(
        team_state
            .request(&workers)
            .expect_err("unknown worker rejected")
            .contains("loaded worker")
    );

    assert!(resource_project_path(None).expect_err("missing project rejected").contains("project"));
}

#[test]
fn staff_resource_states_create_unique_drafts_and_apply_saved_resources() {
    let mut worker_state = StaffWorkerResourcesState::default();
    assert!(worker_state.request().expect_err("missing worker draft").contains("not selected"));
    worker_state.replace_workers(vec![worker("worker"), worker("worker-2")]);
    worker_state.start_new();
    assert_eq!(worker_state.draft.as_ref().expect("new worker draft").worker_id, "worker-3");
    let request = worker_state.request().expect("new worker request");
    assert_eq!(request.worker_id, "worker-3");
    assert_eq!(request.model.as_deref(), Some("auto"));

    let saved_worker = worker("worker-3");
    worker_state.apply_saved(saved_worker.clone());
    assert_eq!(worker_state.selected_worker_id.as_deref(), Some("worker-3"));
    assert_eq!(worker_state.workers.last().map(|worker| worker.id.as_str()), Some("worker-3"));
    assert_eq!(worker_state.success.as_deref(), Some("Worker `worker-3` saved"));

    let mut team_state = StaffTeamResourcesState::default();
    assert!(
        team_state
            .request(&worker_state.workers)
            .expect_err("missing team draft")
            .contains("not selected")
    );
    team_state.replace_teams(vec![team()]);
    team_state.start_new();
    assert_eq!(team_state.draft.as_ref().expect("new team draft").team_id, "team");

    let mut saved_team = team();
    saved_team.id = "support".to_string();
    saved_team.name = "Support".to_string();
    team_state.apply_saved(saved_team.clone());
    assert_eq!(team_state.selected_team_id.as_deref(), Some("support"));
    assert_eq!(team_state.teams.last().map(|team| team.id.as_str()), Some("support"));
    assert_eq!(team_state.success.as_deref(), Some("Team `support` saved"));

    assert_eq!(resource_project_path(Some("  /repo  ")).as_deref(), Ok("/repo"));
    assert!(resource_project_path(Some(" \t ")).is_err());
}

#[test]
fn replacing_and_selecting_workers_preserves_state_boundaries() {
    let mut state = StaffWorkerResourcesState {
        loading: true,
        saving: true,
        error: Some("stale error".to_string()),
        success: Some("stale success".to_string()),
        selected_worker_id: Some("worker-b".to_string()),
        ..StaffWorkerResourcesState::default()
    };

    state.replace_workers(vec![worker("worker-a"), worker("worker-b")]);

    assert_eq!(state.selected_worker_id.as_deref(), Some("worker-b"));
    assert_eq!(state.draft.as_ref().expect("draft").worker_id, "worker-b");
    assert!(!state.loading);
    assert!(!state.saving);
    assert!(state.error.is_none());
    assert!(state.success.is_none());

    state.error = Some("keep error".to_string());
    state.success = Some("keep success".to_string());
    state.select("missing-worker");
    assert_eq!(state.selected_worker_id.as_deref(), Some("worker-b"));
    assert_eq!(state.error.as_deref(), Some("keep error"));
    assert_eq!(state.success.as_deref(), Some("keep success"));

    state.replace_workers(vec![worker("worker-a")]);
    assert_eq!(state.selected_worker_id.as_deref(), Some("worker-a"));
    state.replace_workers(Vec::new());
    assert!(state.selected_worker_id.is_none());
    assert!(state.draft.is_none());
}

#[test]
fn worker_request_trims_optional_fields_and_reports_required_fields() {
    let mut state = StaffWorkerResourcesState::default();
    state.replace_workers(vec![worker("worker-a")]);
    let draft = state.draft.as_mut().expect("draft");
    draft.worker_id = " worker-a ".to_string();
    draft.model = " gpt-5.2 ".to_string();
    draft.runtime = "   ".to_string();
    draft.deployment = " desktop ".to_string();
    draft.team_id = " platform ".to_string();

    let request = state.request().expect("trimmed request");

    assert_eq!(request.worker_id, "worker-a");
    assert_eq!(request.model.as_deref(), Some("gpt-5.2"));
    assert_eq!(request.runtime, None);
    assert_eq!(request.deployment.as_deref(), Some("desktop"));
    assert_eq!(request.team_id.as_deref(), Some("platform"));

    state.draft.as_mut().expect("draft").model = " ".to_string();
    assert_eq!(state.request().unwrap_err(), "worker model is required");
    state.draft.as_mut().expect("draft").worker_id = " ".to_string();
    state.draft.as_mut().expect("draft").model = "model".to_string();
    assert_eq!(state.request().unwrap_err(), "worker id is required");
}

#[test]
fn replacing_and_selecting_teams_preserves_state_boundaries() {
    let mut state = StaffTeamResourcesState {
        loading: true,
        saving: true,
        error: Some("stale error".to_string()),
        success: Some("stale success".to_string()),
        selected_team_id: Some("platform".to_string()),
        ..StaffTeamResourcesState::default()
    };
    let mut support = team();
    support.id = "support".to_string();
    support.name = "Support".to_string();

    state.replace_teams(vec![team(), support]);

    assert_eq!(state.selected_team_id.as_deref(), Some("platform"));
    assert_eq!(state.draft.as_ref().expect("draft").team_id, "platform");
    assert!(!state.loading);
    assert!(!state.saving);
    assert!(state.error.is_none());
    assert!(state.success.is_none());

    state.error = Some("keep error".to_string());
    state.success = Some("keep success".to_string());
    state.select("missing-team");
    assert_eq!(state.selected_team_id.as_deref(), Some("platform"));
    assert_eq!(state.error.as_deref(), Some("keep error"));
    assert_eq!(state.success.as_deref(), Some("keep success"));

    state.replace_teams(Vec::new());
    assert!(state.selected_team_id.is_none());
    assert!(state.draft.is_none());
}

#[test]
fn team_request_reports_required_fields_and_member_field_errors() {
    let workers = vec![worker("worker-a")];
    let mut state = StaffTeamResourcesState::default();
    state.start_new();
    let draft = state.draft.as_mut().expect("draft");
    draft.team_id = " platform ".to_string();
    draft.name = " Core ".to_string();
    draft.leader_id = " worker-a ".to_string();
    draft.members = vec![StaffTeamMemberDraft {
        worker_id: " worker-a ".to_string(),
        runtime_name: " matrix ".to_string(),
        role: " lead ".to_string(),
        room_id: " !room ".to_string(),
        matrix_user_id: " @worker:example.test ".to_string(),
        ready: true,
    }];

    let request = state.request(&workers).expect("trimmed team request");

    assert_eq!(request.team_id, "platform");
    assert_eq!(request.name, "Core");
    assert_eq!(request.leader_id, "worker-a");
    assert_eq!(request.members[0].runtime_name.as_deref(), Some("matrix"));
    assert_eq!(request.members[0].role, "lead");
    assert_eq!(request.members[0].room_id.as_deref(), Some("!room"));

    state.draft.as_mut().expect("draft").members[0].role = " ".to_string();
    assert_eq!(state.request(&workers).unwrap_err(), "team member role is required");
    state.draft.as_mut().expect("draft").members[0].role = "lead".to_string();
    state.draft.as_mut().expect("draft").team_id = " ".to_string();
    assert_eq!(state.request(&workers).unwrap_err(), "team id is required");
    state.draft.as_mut().expect("draft").team_id = "platform".to_string();
    state.draft.as_mut().expect("draft").name = " ".to_string();
    assert_eq!(state.request(&workers).unwrap_err(), "team name is required");
}

#[test]
fn member_draft_and_trim_helpers_keep_expected_defaults() {
    let member = StaffTeamMemberDraft::new("worker-a".to_string());

    assert_eq!(member.worker_id, "worker-a");
    assert_eq!(member.role, "member");
    assert!(!member.ready);
    assert_eq!(required_trimmed(" value ", "field").as_deref(), Ok("value"));
    assert_eq!(required_trimmed(" ", "field").unwrap_err(), "field is required");
    assert_eq!(optional_trimmed(" value ").as_deref(), Some("value"));
    assert_eq!(optional_trimmed(" "), None);
    assert_eq!(next_resource_id("worker", ["worker", "worker-3"].into_iter()), "worker-2");
    assert_eq!(next_resource_id("team", ["other"].into_iter()), "team");
}
