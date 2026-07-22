use std::collections::HashSet;

use super::events::AgentEventKind;
use super::state_machine::{
    AgentTaskKind, AgentTaskStatus, agent_task_can_transition, ensure_transition,
    ensure_transition_key,
};

#[test]
fn agent_state_machine_accepts_finite_infinite_and_team_transitions() {
    let finite_path = [
        (AgentTaskStatus::Assigned, AgentTaskStatus::Dispatched),
        (AgentTaskStatus::Dispatched, AgentTaskStatus::Acknowledged),
        (AgentTaskStatus::Acknowledged, AgentTaskStatus::Running),
        (AgentTaskStatus::Running, AgentTaskStatus::Submitted),
        (AgentTaskStatus::Submitted, AgentTaskStatus::Completed),
    ];

    for (from, to) in finite_path {
        ensure_transition(AgentTaskKind::Finite, from, to).expect("finite transition allowed");
    }

    assert!(!agent_task_can_transition(
        AgentTaskKind::Finite,
        AgentTaskStatus::Completed,
        AgentTaskStatus::Running,
    ));
    let error = ensure_transition(
        AgentTaskKind::Finite,
        AgentTaskStatus::Completed,
        AgentTaskStatus::Running,
    )
    .expect_err("completed finite task must not restart");
    assert!(error.to_string().contains("finite"));
    assert!(error.to_string().contains("completed -> running"));

    let infinite_path = [
        (AgentTaskStatus::Active, AgentTaskStatus::Dispatched),
        (AgentTaskStatus::Dispatched, AgentTaskStatus::Acknowledged),
        (AgentTaskStatus::Acknowledged, AgentTaskStatus::Running),
        (AgentTaskStatus::Running, AgentTaskStatus::Active),
    ];

    for (from, to) in infinite_path {
        ensure_transition(AgentTaskKind::Infinite, from, to)
            .expect("infinite execution cycle transition allowed");
    }

    let error = ensure_transition(
        AgentTaskKind::Infinite,
        AgentTaskStatus::Running,
        AgentTaskStatus::Completed,
    )
    .expect_err("infinite execution must return to active instead of completing");
    assert!(error.to_string().contains("infinite"));
    assert!(error.to_string().contains("running -> completed"));

    let team_node_path = [
        (AgentTaskStatus::Queued, AgentTaskStatus::Assigned),
        (AgentTaskStatus::Assigned, AgentTaskStatus::Dispatched),
        (AgentTaskStatus::Dispatched, AgentTaskStatus::Acknowledged),
        (AgentTaskStatus::Acknowledged, AgentTaskStatus::Running),
        (AgentTaskStatus::Running, AgentTaskStatus::Submitted),
        (AgentTaskStatus::Submitted, AgentTaskStatus::Completed),
    ];

    for (from, to) in team_node_path {
        ensure_transition(AgentTaskKind::TeamDag, from, to)
            .expect("team DAG node transition allowed");
    }
}

#[test]
fn agent_state_machine_allows_idempotent_transitions() {
    for kind in [AgentTaskKind::Finite, AgentTaskKind::Infinite, AgentTaskKind::TeamDag] {
        for status in
            [AgentTaskStatus::Assigned, AgentTaskStatus::Running, AgentTaskStatus::Completed]
        {
            ensure_transition(kind, status, status).expect("idempotent transition allowed");
        }
    }
}

#[test]
fn agent_state_machine_event_kind_catalog_uses_stable_snake_case_names() {
    let required = [
        "task_created",
        "imported_legacy_task",
        "worker_selected",
        "matrix_dispatched",
        "worker_acknowledged",
        "run_completed",
        "human_intervened",
    ];
    let catalog = AgentEventKind::catalog();
    let names = catalog.iter().map(|kind| kind.as_str()).collect::<HashSet<_>>();

    for kind in required {
        assert!(names.contains(kind), "missing event kind {kind}");
    }
    assert_eq!(names.len(), catalog.len(), "event kind names must be unique");
    assert!(names.iter().all(|kind| kind.chars().all(|ch| ch.is_ascii_lowercase() || ch == '_')));
}

#[test]
fn agent_state_machine_transition_keys_parse_and_reject_invalid_inputs() {
    ensure_transition_key(AgentTaskKind::Finite, "draft", "assigned")
        .expect("finite transition keys are accepted");

    let invalid_status = ensure_transition_key(AgentTaskKind::Finite, "unknown", "queued")
        .expect_err("unknown status key must be rejected");
    assert_eq!(invalid_status.to_string(), "unsupported agent task status: unknown");

    let invalid_transition = ensure_transition_key(AgentTaskKind::TeamDag, "completed", "running")
        .expect_err("completed team DAG task must not restart");
    assert!(invalid_transition.to_string().contains("team_dag"));
    assert!(invalid_transition.to_string().contains("completed -> running"));
}
