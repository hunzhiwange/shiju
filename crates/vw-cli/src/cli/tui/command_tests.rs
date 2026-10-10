use std::net::TcpListener;
use std::path::PathBuf;

use vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerScopeDto;
use vw_gateway_client::vw_api_types::id::ProjectId;
use vw_gateway_client::{CronJobDto, GatewayClient, GatewayEndpoint};
use vw_tui::{V3Action, V3ApprovalDecision};

use super::app::{ActiveRunner, V3GatewayApp};
use super::bridge::GatewayV3Bridge;
use super::commands::{
    CommandRoute, WORKFLOW_SELECTION_SURFACE, WORKSPACE_FILES_SURFACE, command_palette_items,
    route_selection, route_slash_submission,
};
use super::controller::{TuiController, TuiControllerCommand};
use super::model::background_task::{
    BackgroundTask, BackgroundTaskKind, BackgroundTaskPanel, BackgroundTaskStatus,
    CanonicalTaskStatus, TaskControlAction,
};
use super::model::schedule_panel::{SchedulePanel, SchedulePanelItem};
use super::model::workflow_panel::{WorkflowPanel, WorkflowPanelItem};
use super::model::{UiConfirmIntent, UiOverlay};
use super::runtime::cron::CronJobSnapshot;
use super::runtime::gateway::{GatewaySessionSeed, GatewayUiRuntime};
use super::runtime::orchestration::OrchestrationSnapshot;
use super::runtime::workflow_run::workflow_run_request;
use super::state::TuiState;

#[test]
fn selected_command_is_never_silently_ignored() {
    let mut bridge = GatewayV3Bridge::default();
    let mut state = TuiState::default();
    let mut controller = TuiController;

    let command = bridge
        .controller_command(
            &mut state,
            &mut controller,
            V3Action::Select {
                surface: "command_palette".to_string(),
                item_id: "help".to_string(),
            },
        )
        .expect("a known stable command id must be routed");
    assert_eq!(command, TuiControllerCommand::Command(CommandRoute::Help));

    let unknown = bridge
        .controller_command(
            &mut state,
            &mut controller,
            V3Action::Select {
                surface: "command_palette".to_string(),
                item_id: "not-registered".to_string(),
            },
        )
        .expect_err("an unknown command id must be visible to the user");
    assert!(unknown.contains("not-registered"));
}

#[test]
fn command_catalog_exposes_stable_ids_and_disabled_reasons() {
    let items = command_palette_items();
    let ids = items.iter().map(|item| item.id.as_str()).collect::<Vec<_>>();
    assert_eq!(
        &ids[..3],
        &["help", "status", "exit"],
        "the locally implemented command ids remain stable"
    );
    assert!(items.iter().any(|item| item.id == "resume" && !item.disabled));
    assert!(items.iter().any(|item| item.id == "continue" && !item.disabled));
    assert!(items.iter().any(|item| item.id == "new" && !item.disabled));
}

#[test]
fn diff_command_routes_to_session_diff() {
    let items = command_palette_items();
    assert!(items.iter().any(|item| item.id == "diff" && !item.disabled));

    assert_eq!(
        route_slash_submission("/diff").expect("/diff should route"),
        Some(CommandRoute::ShowDiff)
    );

    let mut bridge = GatewayV3Bridge::default();
    let mut state = TuiState::default();
    let mut controller = TuiController;
    let selected = bridge
        .controller_command(
            &mut state,
            &mut controller,
            V3Action::Select {
                surface: "command_palette".to_string(),
                item_id: "diff".to_string(),
            },
        )
        .expect("command palette /diff should route");
    assert_eq!(selected, TuiControllerCommand::Command(CommandRoute::ShowDiff));
}

#[test]
fn unknown_slash_command_is_rejected_without_a_gateway_request() {
    let mut bridge = GatewayV3Bridge::default();
    let mut state = TuiState::default();
    let mut controller = TuiController;

    let error = bridge
        .controller_command(
            &mut state,
            &mut controller,
            V3Action::Submit { text: "/does-not-exist".to_string(), attachments: Vec::new() },
        )
        .expect_err("unknown slash input must not be submitted as chat");
    assert!(error.contains("does-not-exist"));

    let local = TuiControllerCommand::Command(CommandRoute::Status);
    assert!(GatewayV3Bridge::request_for_command(&state, &local).is_none());
}

#[test]
fn cd_command_routes_non_empty_path() {
    assert_eq!(
        route_slash_submission("/cd ../target directory").expect("/cd should route"),
        Some(CommandRoute::ChangeDirectory { path: "../target directory".to_string() })
    );
    assert!(
        route_slash_submission("/cd   ")
            .expect_err("/cd requires a non-empty path")
            .contains("non-empty path")
    );
}

#[test]
fn doctor_command_routes_to_gateway_diagnostics() {
    assert_eq!(
        route_slash_submission("/doctor").expect("/doctor should route"),
        Some(CommandRoute::RunDoctor)
    );
    let doctor = command_palette_items()
        .into_iter()
        .find(|item| item.id == "doctor")
        .expect("doctor command should be discoverable");
    assert!(!doctor.disabled, "doctor command should be enabled");
}

#[test]
fn files_command_routes_relative_path() {
    assert_eq!(
        route_slash_submission("/files").expect("/files should route"),
        Some(CommandRoute::OpenWorkspaceFiles { path: ".".to_string() })
    );
    assert_eq!(
        route_slash_submission("/files src/components")
            .expect("/files should accept a relative directory"),
        Some(CommandRoute::OpenWorkspaceFiles { path: "src/components".to_string() })
    );
    assert_eq!(
        route_selection(WORKSPACE_FILES_SURFACE, "directory:src")
            .expect("directory selections should continue browsing"),
        CommandRoute::OpenWorkspaceFiles { path: "src".to_string() }
    );
    assert_eq!(
        route_selection(WORKSPACE_FILES_SURFACE, "file:src/lib.rs")
            .expect("file selections should read the selected file"),
        CommandRoute::ReadWorkspaceFile { path: "src/lib.rs".to_string() }
    );
    for command in ["/files ../secret", "/files /etc", r"/files C:\secret"] {
        assert!(
            route_slash_submission(command).is_err(),
            "workspace escape must be rejected: {command}"
        );
    }
}

#[test]
fn hooks_commands_route_catalog_audit_and_mutations() {
    assert_eq!(
        route_slash_submission("/hooks").expect("/hooks should route"),
        Some(CommandRoute::OpenHookCatalog)
    );
    assert_eq!(
        route_slash_submission("/hooks audit").expect("/hooks audit should route"),
        Some(CommandRoute::ShowHookAudit)
    );
    assert_eq!(
        route_slash_submission("/hooks trust plugin hash").expect("trust should route"),
        Some(CommandRoute::ConfirmHookMutation {
            plugin_id: "plugin".to_string(),
            hook_hash: "hash".to_string(),
            trusted: true,
        })
    );
    assert_eq!(
        route_slash_submission("/hooks revoke plugin hash").expect("revoke should route"),
        Some(CommandRoute::ConfirmHookMutation {
            plugin_id: "plugin".to_string(),
            hook_hash: "hash".to_string(),
            trusted: false,
        })
    );
    for command in [
        "/hooks trust",
        "/hooks trust plugin",
        "/hooks trust plugin hash extra",
        "/hooks remove plugin hash",
    ] {
        assert!(route_slash_submission(command).is_err(), "invalid Hook mutation: {command}");
    }
    assert!(
        command_palette_items().iter().any(|item| item.id == "hooks" && !item.disabled),
        "/hooks should be discoverable"
    );
}

#[test]
fn memory_command_preserves_query() {
    assert_eq!(
        route_slash_submission("/memory rust async").expect("memory query should route"),
        Some(CommandRoute::QueryMemory { query: Some("rust async".to_string()) })
    );
    assert_eq!(
        route_slash_submission("/memory").expect("empty memory query should refresh the directory"),
        Some(CommandRoute::QueryMemory { query: None })
    );
}

#[test]
fn memory_mutation_parser_requires_key_and_content() {
    assert_eq!(
        route_slash_submission("/memory remember project/key keep this exact content")
            .expect("memory store should route"),
        Some(CommandRoute::ConfirmMemoryStore {
            key: "project/key".to_string(),
            content: "keep this exact content".to_string(),
        })
    );
    assert_eq!(
        route_slash_submission("/memory forget project/key").expect("memory delete should route"),
        Some(CommandRoute::ConfirmMemoryDelete { key: "project/key".to_string() })
    );

    for command in [
        "/memory remember",
        "/memory remember project/key",
        "/memory forget",
        "/memory forget project/key extra",
    ] {
        assert!(
            route_slash_submission(command).is_err(),
            "invalid memory mutation must be rejected: {command}"
        );
    }
}

#[test]
fn workflow_selection_routes_current_uuid() {
    assert_eq!(
        route_selection(WORKFLOW_SELECTION_SURFACE, " wf/current ")
            .expect("current workflow UUID should route"),
        CommandRoute::OpenWorkflowDetail { uuid: "wf/current".to_string() }
    );
    assert!(
        route_selection(WORKFLOW_SELECTION_SURFACE, "   ")
            .expect_err("blank workflow UUID must be rejected")
            .contains("non-empty stable UUID")
    );
}

#[test]
fn workflow_dynamic_command_requires_catalog_uuid_and_query() {
    assert_eq!(
        route_slash_submission("/workflow:wf/current deploy safely")
            .expect("owned Workflow command should parse"),
        Some(CommandRoute::StartWorkflowRun {
            uuid: "wf/current".to_string(),
            query: "deploy safely".to_string(),
        })
    );
    assert!(
        route_slash_submission("/workflow:wf/current   ")
            .expect_err("Workflow command requires a query")
            .contains("non-empty query")
    );

    let snapshot = OrchestrationSnapshot {
        workflows: WorkflowPanel {
            items: vec![WorkflowPanelItem {
                uuid: "wf/current".to_string(),
                name: "Deploy".to_string(),
                description: "Ship safely".to_string(),
            }],
            ..WorkflowPanel::default()
        },
        ..OrchestrationSnapshot::default()
    };
    let request =
        workflow_run_request(&snapshot, Some("stable-session"), "wf/current", "deploy safely")
            .expect("owned UUID and stable session should build a request");
    assert_eq!(request.uuid, "wf/current");
    assert_eq!(request.body.query, "deploy safely");
    assert_eq!(request.body.user, "stable-session");

    let error =
        workflow_run_request(&snapshot, Some("stable-session"), "wf/stale", "deploy safely")
            .expect_err("stale Workflow UUID must be rejected before a request");
    assert!(error.contains("current Workflow snapshot"));
}

#[test]
fn task_control_routes_supported_commands_only() {
    for (command_name, action) in [
        ("start", TaskControlAction::Start),
        ("retry", TaskControlAction::Retry),
        ("pause", TaskControlAction::Pause),
        ("resume", TaskControlAction::Resume),
        ("cancel", TaskControlAction::Cancel),
        ("archive", TaskControlAction::Archive),
    ] {
        assert_eq!(
            route_slash_submission(&format!("/tasks {command_name} task/owned"))
                .expect("supported task control should route"),
            Some(CommandRoute::ConfirmTaskControl { id: "task/owned".to_string(), action })
        );
    }

    for command in [
        "/tasks start",
        "/tasks pause task/owned extra",
        "/tasks promote task/owned",
        "/tasks merge task/owned",
        "/tasks move task/owned",
        "/tasks update-metadata task/owned",
    ] {
        assert!(route_slash_submission(command).is_err(), "unsupported task control: {command}");
    }
}

#[test]
fn task_control_enforces_canonical_transition_and_confirmation() {
    let actions = [
        TaskControlAction::Start,
        TaskControlAction::Retry,
        TaskControlAction::Pause,
        TaskControlAction::Resume,
        TaskControlAction::Cancel,
        TaskControlAction::Archive,
    ];
    let statuses = [
        CanonicalTaskStatus::Draft,
        CanonicalTaskStatus::Queued,
        CanonicalTaskStatus::Assigned,
        CanonicalTaskStatus::Dispatched,
        CanonicalTaskStatus::Acknowledged,
        CanonicalTaskStatus::Active,
        CanonicalTaskStatus::Running,
        CanonicalTaskStatus::Submitted,
        CanonicalTaskStatus::Revision,
        CanonicalTaskStatus::Blocked,
        CanonicalTaskStatus::Paused,
        CanonicalTaskStatus::Completed,
        CanonicalTaskStatus::Failed,
        CanonicalTaskStatus::Cancelled,
        CanonicalTaskStatus::Archived,
    ];
    for status in statuses {
        for action in actions {
            let expected = match action {
                TaskControlAction::Start => {
                    matches!(status, CanonicalTaskStatus::Draft | CanonicalTaskStatus::Queued)
                }
                TaskControlAction::Retry => status == CanonicalTaskStatus::Failed,
                TaskControlAction::Pause => matches!(
                    status,
                    CanonicalTaskStatus::Assigned
                        | CanonicalTaskStatus::Dispatched
                        | CanonicalTaskStatus::Acknowledged
                        | CanonicalTaskStatus::Active
                        | CanonicalTaskStatus::Running
                ),
                TaskControlAction::Resume => status == CanonicalTaskStatus::Paused,
                TaskControlAction::Cancel => matches!(
                    status,
                    CanonicalTaskStatus::Draft
                        | CanonicalTaskStatus::Queued
                        | CanonicalTaskStatus::Assigned
                        | CanonicalTaskStatus::Dispatched
                        | CanonicalTaskStatus::Acknowledged
                        | CanonicalTaskStatus::Active
                        | CanonicalTaskStatus::Running
                        | CanonicalTaskStatus::Submitted
                        | CanonicalTaskStatus::Blocked
                        | CanonicalTaskStatus::Paused
                ),
                TaskControlAction::Archive => matches!(
                    status,
                    CanonicalTaskStatus::Completed
                        | CanonicalTaskStatus::Failed
                        | CanonicalTaskStatus::Cancelled
                ),
            };
            assert_eq!(
                status.allows(action),
                expected,
                "unexpected {action:?} support for {status:?}"
            );
        }
    }

    let listener = TcpListener::bind("127.0.0.1:0").expect("zero-request fixture");
    listener.set_nonblocking(true).expect("nonblocking fixture");
    let port = listener.local_addr().expect("fixture address").port();
    let runtime = GatewayUiRuntime::new(
        GatewayClient::new(GatewayEndpoint::new("127.0.0.1", port)).expect("client"),
        GatewaySessionSeed::new(PathBuf::from("/workspace"))
            .with_id(Some("task-command-session".to_string())),
    );
    let mut state = TuiState::default();
    state.session.session_id = Some("task-command-session".to_string());
    state.orchestration.scope = Some(
        AgentSchedulerScopeDto::for_default_tenant(ProjectId::from("project-one"))
            .expect("task scope"),
    );
    state.orchestration.tasks = BackgroundTaskPanel {
        tasks: vec![BackgroundTask {
            id: "task/owned".to_string(),
            session_id: "project-one".to_string(),
            kind: BackgroundTaskKind::Agent,
            title: "Owned task".to_string(),
            status: BackgroundTaskStatus::Running,
            canonical_status: CanonicalTaskStatus::Running,
            detail: "running".to_string(),
            log: Vec::new(),
        }],
        ..BackgroundTaskPanel::default()
    };
    let mut app = V3GatewayApp::new(runtime, state);
    let mut runner = ActiveRunner::new_with_editor(std::sync::Arc::new(|_| Ok(None)));

    runner.execute_action(
        &mut app,
        V3Action::Submit { text: "/tasks pause task/owned".to_string(), attachments: Vec::new() },
    );
    let UiOverlay::Confirm(pause) = app.state().overlays.active().expect("pause confirmation")
    else {
        panic!("expected typed task confirmation");
    };
    assert!(!pause.destructive, "pause must not be marked destructive");
    assert_eq!(
        pause.intent,
        UiConfirmIntent::TaskControl {
            id: "task/owned".to_string(),
            action: TaskControlAction::Pause,
        }
    );
    let request_id = pause.request_id.clone();
    runner.execute_action(
        &mut app,
        V3Action::ResolveApproval { request_id, decision: V3ApprovalDecision::Cancel },
    );

    for command in ["/tasks retry task/owned", "/tasks pause task/stale"] {
        runner.execute_action(
            &mut app,
            V3Action::Submit { text: command.to_string(), attachments: Vec::new() },
        );
        assert!(
            !matches!(app.state().overlays.active(), Some(UiOverlay::Confirm(_))),
            "illegal or unknown task control must not open confirmation: {command}"
        );
    }
    assert!(
        matches!(listener.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock),
        "cancelled, illegal, and unknown task controls must issue zero Gateway requests"
    );
}

#[test]
fn schedule_mutations_require_snapshot_id_and_confirmation() {
    assert_eq!(
        route_slash_submission("/schedule enable cron/owned").expect("enable should route"),
        Some(CommandRoute::ConfirmCronToggle { job_id: "cron/owned".to_string(), enabled: true })
    );
    assert_eq!(
        route_slash_submission("/schedule disable cron/owned").expect("disable should route"),
        Some(CommandRoute::ConfirmCronToggle { job_id: "cron/owned".to_string(), enabled: false })
    );
    assert_eq!(
        route_slash_submission("/schedule delete cron/owned").expect("delete should route"),
        Some(CommandRoute::ConfirmCronDelete { job_id: "cron/owned".to_string() })
    );
    for command in
        ["/schedule enable", "/schedule delete cron/owned extra", "/schedule edit cron/owned"]
    {
        assert!(route_slash_submission(command).is_err(), "invalid Cron mutation: {command}");
    }

    let listener = TcpListener::bind("127.0.0.1:0").expect("zero-request fixture");
    listener.set_nonblocking(true).expect("nonblocking fixture");
    let port = listener.local_addr().expect("fixture address").port();
    let runtime = GatewayUiRuntime::new(
        GatewayClient::new(GatewayEndpoint::new("127.0.0.1", port)).expect("client"),
        GatewaySessionSeed::new(PathBuf::from("/workspace"))
            .with_id(Some("cron-command-session".to_string())),
    );
    let job = command_test_cron_job("cron/owned");
    let mut state = TuiState::default();
    state.session.session_id = Some("cron-command-session".to_string());
    state.orchestration.schedules = SchedulePanel {
        items: vec![SchedulePanelItem {
            id: job.id.clone(),
            name: job.name.clone().expect("name"),
            schedule: job.expression.clone(),
            next_run: job.next_run.clone(),
            last_status: job.last_status.clone(),
            enabled: job.enabled,
            history: Vec::new(),
            history_error: None,
        }],
        ..SchedulePanel::default()
    };
    state.orchestration.cron_jobs = vec![CronJobSnapshot::from_dto(&job)];
    let mut app = V3GatewayApp::new(runtime, state);
    let mut runner = ActiveRunner::new_with_editor(std::sync::Arc::new(|_| Ok(None)));

    runner.execute_action(
        &mut app,
        V3Action::Submit {
            text: "/schedule disable cron/owned".to_string(),
            attachments: Vec::new(),
        },
    );
    let UiOverlay::Confirm(disable) = app.state().overlays.active().expect("confirmation") else {
        panic!("expected typed Cron confirmation");
    };
    assert!(!disable.destructive, "only Cron deletion is destructive");
    assert_eq!(
        disable.intent,
        UiConfirmIntent::ScheduleToggle { id: "cron/owned".to_string(), enabled: false }
    );
    let request_id = disable.request_id.clone();
    runner.execute_action(
        &mut app,
        V3Action::ResolveApproval { request_id, decision: V3ApprovalDecision::Cancel },
    );
    assert!(app.state().overlays.active().is_none());

    runner.execute_action(
        &mut app,
        V3Action::Submit {
            text: "/schedule delete cron/owned".to_string(),
            attachments: Vec::new(),
        },
    );
    let UiOverlay::Confirm(delete) = app.state().overlays.active().expect("delete confirmation")
    else {
        panic!("expected typed Cron delete confirmation");
    };
    assert!(delete.destructive);
    assert_eq!(delete.intent, UiConfirmIntent::ScheduleDelete { id: "cron/owned".to_string() });
    let request_id = delete.request_id.clone();
    runner.execute_action(
        &mut app,
        V3Action::ResolveApproval { request_id, decision: V3ApprovalDecision::Cancel },
    );

    runner.execute_action(
        &mut app,
        V3Action::Submit {
            text: "/schedule enable cron/stale".to_string(),
            attachments: Vec::new(),
        },
    );
    assert!(!matches!(app.state().overlays.active(), Some(UiOverlay::Confirm(_))));
    assert!(
        matches!(listener.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock),
        "unknown and cancelled Cron mutations must issue zero Gateway requests"
    );
}

fn command_test_cron_job(id: &str) -> CronJobDto {
    CronJobDto {
        id: id.to_string(),
        name: Some("Nightly".to_string()),
        job_type: "command".to_string(),
        schedule_kind: "cron".to_string(),
        expression: "0 0 * * *".to_string(),
        at: None,
        every_ms: None,
        command: "deploy".to_string(),
        prompt: None,
        model: None,
        agent: None,
        acp_agent: None,
        project_path: Some("/workspace".to_string()),
        wake: false,
        fallbacks: Vec::new(),
        full_access: false,
        task_pool: false,
        delivery_mode: "none".to_string(),
        delivery_channel: None,
        delivery_to: None,
        delivery_best_effort: false,
        delete_after_run: false,
        next_run: "tomorrow".to_string(),
        last_run: None,
        last_status: None,
        last_output: None,
        enabled: true,
    }
}
