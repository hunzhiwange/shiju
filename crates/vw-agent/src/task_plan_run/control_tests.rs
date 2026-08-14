use super::PlanRunControlRegistry;
use crate::session::child_agent::coordinator::ChildAgentCoordinator;
use vw_api_types::id::TenantScope;
use vw_api_types::task::{TaskPlanRunStatus, TaskPlanRunStreamEventDto};

#[test]
fn task_plan_run_stop_is_tenant_scoped_and_cancels_descendants() {
    let registry = PlanRunControlRegistry::new(8);
    let owner = TenantScope::new("platform-owner", "company-owner").expect("owner tenant");
    let outsider =
        TenantScope::new("platform-outsider", "company-outsider").expect("outsider tenant");
    let task_id = "tenant-scoped-plan-run";
    let root_session_id = "tenant-scoped-plan-run-plan";
    let child_session_id = "tenant-scoped-plan-run-child";
    let grandchild_session_id = "tenant-scoped-plan-run-grandchild";

    let root_cancellation =
        registry.register(owner.clone(), task_id).expect("register plan-run control");
    registry
        .observe_event(
            &owner,
            task_id,
            &TaskPlanRunStreamEventDto::PlanningStarted {
                task_id: task_id.to_string(),
                plan_dir: "/repo/.plans/task-tenant-scoped-plan-run".to_string(),
                session_id: root_session_id.to_string(),
            },
        )
        .expect("record planning session");

    let coordinator = ChildAgentCoordinator::new(8);
    let child_cancellation = coordinator
        .register(root_session_id, child_session_id, "planner", "delegate validation")
        .expect("register child");
    coordinator.mark_running(root_session_id, child_session_id).expect("start child");
    let grandchild_cancellation = coordinator
        .register(child_session_id, grandchild_session_id, "reviewer", "validate result")
        .expect("register grandchild");
    coordinator.mark_running(child_session_id, grandchild_session_id).expect("start grandchild");
    registry
        .observe_event(
            &owner,
            task_id,
            &TaskPlanRunStreamEventDto::SessionWaiting {
                task_id: task_id.to_string(),
                session_id: root_session_id.to_string(),
                active_descendant_session_ids: vec![
                    child_session_id.to_string(),
                    grandchild_session_id.to_string(),
                ],
            },
        )
        .expect("record descendant wait");
    assert_eq!(
        registry.status(&owner, task_id).expect("owner status").status,
        TaskPlanRunStatus::Waiting
    );

    assert!(
        registry.status(&outsider, task_id).is_none(),
        "another tenant must not observe the run"
    );
    assert!(
        registry
            .stop(&outsider, task_id, &coordinator)
            .expect("cross-tenant stop lookup")
            .is_none(),
        "another tenant must not stop the run"
    );
    assert!(!root_cancellation.is_cancelled());
    assert!(!child_cancellation.is_cancelled());
    assert!(!grandchild_cancellation.is_cancelled());

    let stopped = registry
        .stop(&owner, task_id, &coordinator)
        .expect("owner stop")
        .expect("owner run must exist");
    assert_eq!(stopped.status.status, TaskPlanRunStatus::Cancelled);
    assert_eq!(stopped.cancelled_descendants, 2);
    assert!(!stopped.already_terminal);
    assert!(root_cancellation.is_cancelled());
    assert!(child_cancellation.is_cancelled());
    assert!(grandchild_cancellation.is_cancelled());

    let stopped_again = registry
        .stop(&owner, task_id, &coordinator)
        .expect("idempotent owner stop")
        .expect("terminal run remains queryable");
    assert_eq!(stopped_again.status.status, TaskPlanRunStatus::Cancelled);
    assert_eq!(stopped_again.cancelled_descendants, 0);
    assert!(stopped_again.already_terminal);
}

#[test]
fn task_plan_run_control_bounds_terminal_snapshots_and_preserves_terminal_stop() {
    let registry = PlanRunControlRegistry::new(1);
    let tenant = TenantScope::new("platform-bounded", "company-bounded").expect("tenant");
    let first_cancellation =
        registry.register(tenant.clone(), "first-run").expect("register first run");
    registry
        .observe_event(
            &tenant,
            "first-run",
            &TaskPlanRunStreamEventDto::Finished {
                task_id: "first-run".to_string(),
                plan_dir: "/repo/.plans/first-run".to_string(),
                status: "completed".to_string(),
                output: "done".to_string(),
                session_id: Some("first-run-plan".to_string()),
            },
        )
        .expect("finish first run");

    let second_cancellation =
        registry.register(tenant.clone(), "second-run").expect("register second run");
    registry
        .observe_event(
            &tenant,
            "second-run",
            &TaskPlanRunStreamEventDto::Blocked {
                task_id: "second-run".to_string(),
                batch: Some(1),
                subtask_id: Some("1".to_string()),
                reason: "blocked".to_string(),
                session_id: "second-run-1".to_string(),
            },
        )
        .expect("block second run");

    assert!(
        registry.status(&tenant, "first-run").is_none(),
        "old terminal snapshots must be evicted"
    );
    let terminal_stop = registry
        .stop(&tenant, "second-run", &ChildAgentCoordinator::new(1))
        .expect("terminal stop lookup")
        .expect("newest terminal run remains");
    assert_eq!(terminal_stop.status.status, TaskPlanRunStatus::Blocked);
    assert!(terminal_stop.already_terminal);
    assert!(!first_cancellation.is_cancelled());
    assert!(!second_cancellation.is_cancelled());
    assert!(
        registry
            .stop(&tenant, "unknown-run", &ChildAgentCoordinator::new(1))
            .expect("unknown lookup")
            .is_none()
    );
}
