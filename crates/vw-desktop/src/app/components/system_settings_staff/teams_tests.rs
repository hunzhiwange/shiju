use super::*;
use serde_json::Value;
use vw_gateway_client::vw_api_types::agent_scheduler::{
    AgentCapabilityDto, AgentCapabilityKindDto, AgentTeamDto, AgentTeamMemberDto, AgentWorkerDto,
    AgentWorkerStatusDto,
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
fn team_status_labels_round_trip_and_unknown_defaults_to_pending() {
    let cases = [
        (AgentTeamStatusDto::Pending, "待就绪"),
        (AgentTeamStatusDto::Active, "活跃"),
        (AgentTeamStatusDto::Degraded, "降级"),
        (AgentTeamStatusDto::Failed, "失败"),
    ];

    for (status, label) in cases {
        assert_eq!(team_status_label(&status), label);
        assert_eq!(team_status_from_label(label), status);
    }
    assert_eq!(team_status_from_label("未知状态"), AgentTeamStatusDto::Pending);
}

#[test]
fn resource_message_wraps_staff_resource_messages() {
    assert!(matches!(
        resource_message(StaffResourceMessage::Refresh),
        Message::Settings(SettingsMessage::Staff(StaffMessage::Resources(
            StaffResourceMessage::Refresh
        )))
    ));
    assert!(matches!(
        resource_message(StaffResourceMessage::TeamMemberReadyToggled(2, true)),
        Message::Settings(SettingsMessage::Staff(StaffMessage::Resources(
            StaffResourceMessage::TeamMemberReadyToggled(2, true)
        )))
    ));
}

#[test]
fn view_builds_no_project_feedback_and_new_team_states() {
    let (mut app, _) = App::new();
    app.project_path = None;
    app.staff_settings.teams.success = Some("saved".to_string());
    app.staff_settings.teams.error = Some("failed".to_string());

    let _: Element<'_, Message> = view(&app);

    app.staff_settings.teams.start_new();
    let _: Element<'_, Message> = view(&app);
}

#[test]
fn team_field_helpers_build_enabled_and_disabled_controls() {
    let _: Element<'_, Message> =
        team_text_field("Team ID", "项目内稳定标识。", "team-1", StaffTeamDraftField::TeamId);
    let _: Element<'_, Message> =
        member_pick_field(0, vec!["worker-1".to_string()], "worker-1".to_string());
    let _: Element<'_, Message> =
        member_text_field(0, "Role", "reviewer", StaffTeamMemberDraftField::Role);
    let _: Element<'_, Message> = action_button("刷新", true, StaffResourceMessage::Refresh, true);
    let _: Element<'_, Message> =
        action_button("新增 Team", false, StaffResourceMessage::TeamNewRequested, false);
}

#[test]
fn view_builds_selected_team_with_member_rows_and_available_worker_states() {
    let (mut app, _) = App::new();
    app.project_path = Some("/repo/app".to_string());
    app.staff_settings.workers.replace_workers(vec![worker("worker-a"), worker("worker-b")]);
    app.staff_settings.teams.replace_teams(vec![team()]);

    let _: Element<'_, Message> = view(&app);
    let _: Element<'_, Message> = team_list(&app);
    let _: Element<'_, Message> = team_detail(&app, true);
    let _: Element<'_, Message> = team_members_editor(&app);

    app.staff_settings.workers.replace_workers(vec![
        worker("worker-a"),
        worker("worker-b"),
        worker("worker-c"),
    ]);
    let _: Element<'_, Message> = team_members_editor(&app);
}
