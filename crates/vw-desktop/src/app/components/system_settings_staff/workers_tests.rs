use super::*;
use serde_json::Value;
use vw_gateway_client::vw_api_types::agent_scheduler::{AgentWorkerDto, AgentWorkerStatusDto};
use vw_gateway_client::vw_api_types::common::TimestampMs;

fn child_count(element: Element<'_, Message>) -> usize {
    element.as_widget().children().len()
}

fn keep(element: Element<'_, Message>) {
    std::hint::black_box(element);
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

fn empty_draft() -> StaffWorkerDraft {
    StaffWorkerDraft {
        worker_id: "worker-new".to_string(),
        runtime: String::new(),
        deployment: String::new(),
        model: "auto".to_string(),
        model_provider: String::new(),
        team_id: String::new(),
        worker_role: "worker".to_string(),
        matrix_user_id: String::new(),
        room_id: String::new(),
        capabilities: Vec::new(),
        status: None,
        desired_state: None,
        container_state: None,
        idle_since_ms: None,
        last_seen_at_ms: None,
        last_heartbeat_at: None,
        updated_at_ms: None,
    }
}

#[test]
fn resource_message_wraps_worker_resource_messages() {
    assert!(matches!(
        resource_message(StaffResourceMessage::Refresh),
        Message::Settings(SettingsMessage::Staff(StaffMessage::Resources(
            StaffResourceMessage::Refresh
        )))
    ));
    assert!(matches!(
        resource_message(StaffResourceMessage::WorkerSelected("worker-a".to_string())),
        Message::Settings(SettingsMessage::Staff(StaffMessage::Resources(
            StaffResourceMessage::WorkerSelected(selected)
        ))) if selected == "worker-a"
    ));
}

#[test]
fn view_builds_no_project_feedback_and_new_worker_states() {
    let (mut app, _) = App::new();
    app.project_path = None;
    app.staff_settings.workers.success = Some("saved".to_string());
    app.staff_settings.workers.error = Some("failed".to_string());

    assert!(child_count(view(&app)) > 0);
    assert!(child_count(worker_list(&app)) > 0);
    std::hint::black_box(worker_detail(&app, false));

    app.staff_settings.workers.start_new();

    assert!(child_count(view(&app)) > 0);
    std::hint::black_box(worker_detail(&app, false));
}

#[test]
fn view_builds_loading_empty_worker_list() {
    let (mut app, _) = App::new();
    app.project_path = Some("/repo/app".to_string());
    app.staff_settings.workers.loading = true;

    assert!(child_count(view(&app)) > 0);
    assert!(child_count(worker_list(&app)) > 0);
}

#[test]
fn view_builds_selected_worker_with_capability_rows_and_lifecycle_snapshot() {
    let (mut app, _) = App::new();
    app.project_path = Some("/repo/app".to_string());
    app.staff_settings.workers.replace_workers(vec![worker("worker-a"), worker("worker-b")]);

    assert!(child_count(view(&app)) > 0);
    assert!(child_count(worker_list(&app)) > 0);
    assert!(child_count(worker_detail(&app, true)) > 0);

    let draft = app.staff_settings.workers.draft.as_ref().expect("selected worker draft");
    assert!(child_count(capabilities_editor(draft)) > 0);
    assert!(child_count(lifecycle_snapshot(draft)) > 0);

    app.staff_settings.workers.saving = true;
    assert!(child_count(worker_detail(&app, true)) > 0);
}

#[test]
fn worker_helpers_build_enabled_disabled_and_empty_states() {
    keep(worker_text_field(
        "Worker ID",
        "项目内稳定标识。",
        "worker-a",
        StaffWorkerDraftField::WorkerId,
    ));
    keep(field_label("Runtime", "运行时名称，可留空。"));
    keep(centered_state("暂无 Worker；可新建项目级注册配置。"));
    keep(action_button("刷新", true, StaffResourceMessage::Refresh, true));
    keep(action_button("新增 Worker", false, StaffResourceMessage::WorkerNewRequested, false));
}

#[test]
fn capability_and_lifecycle_helpers_handle_empty_optional_values() {
    let draft = empty_draft();

    assert!(child_count(capabilities_editor(&draft)) > 0);
    assert!(child_count(lifecycle_snapshot(&draft)) > 0);
    keep(readonly_row("Status", None));
    keep(readonly_row("Last Seen", Some("   ".to_string())));
    keep(readonly_row("Updated At", Some("103".to_string())));
}

#[test]
fn capability_kind_keys_round_trip_all_supported_variants() {
    let cases = [
        ("skill", AgentCapabilityKindDto::Skill),
        ("remote_skill", AgentCapabilityKindDto::RemoteSkill),
        ("mcp_server", AgentCapabilityKindDto::McpServer),
        ("model", AgentCapabilityKindDto::Model),
        ("runtime", AgentCapabilityKindDto::Runtime),
        ("tool", AgentCapabilityKindDto::Tool),
    ];

    for (key, kind) in cases {
        assert_eq!(capability_kind_key(&kind), key);
        assert_eq!(capability_kind_key(&capability_kind_from_key(key)), key);
    }
}

#[test]
fn unknown_capability_kind_keys_fall_back_to_tool() {
    for key in ["", "Tool", "remote-skill", "unknown"] {
        assert!(matches!(capability_kind_from_key(key), AgentCapabilityKindDto::Tool));
    }
}

#[test]
fn capability_kind_catalog_matches_gateway_keys() {
    assert_eq!(
        CAPABILITY_KINDS,
        &[
            ("Skill", "skill"),
            ("Remote Skill", "remote_skill"),
            ("MCP Server", "mcp_server"),
            ("Model", "model"),
            ("Runtime", "runtime"),
            ("Tool", "tool"),
        ]
    );
}
