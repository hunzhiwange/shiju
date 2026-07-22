use crate::app::task::{Task, TaskBoardStaffCommand, TaskBoardStaffCommandAction, TaskStatus};

fn staff_task(id: &str, status: TaskStatus, priority: u32, order: u32) -> Task {
    let mut task = Task::default();
    task.id = id.to_string();
    task.status = status;
    task.priority = priority;
    task.order = order;
    task.description = format!("Task {id}");
    task
}

#[test]
fn routing_assigns_tasks_to_role_capacity() {
    let mut config = super::TaskBoardStaffConfig::default();
    config.members = vec![
        super::TaskBoardStaffMember::new(
            "orch-full",
            "Orchestrator Full",
            super::TaskBoardStaffRole::Orchestrator,
        ),
        super::TaskBoardStaffMember::new(
            "orch-free",
            "Orchestrator Free",
            super::TaskBoardStaffRole::Orchestrator,
        ),
        super::TaskBoardStaffMember::new(
            "planner-disabled",
            "Planner Disabled",
            super::TaskBoardStaffRole::Planner,
        ),
        super::TaskBoardStaffMember::new(
            "executor",
            "Executor",
            super::TaskBoardStaffRole::Executor,
        ),
        super::TaskBoardStaffMember::new(
            "reviewer",
            "Reviewer",
            super::TaskBoardStaffRole::Reviewer,
        ),
        super::TaskBoardStaffMember::new("merger", "Merger", super::TaskBoardStaffRole::Merger),
    ];
    config.members[0].capacity = 1;
    config.members[0].current_task_ids = vec!["external-running".to_string()];
    config.members[2].enabled = false;

    let tasks = vec![
        staff_task("T-pool", TaskStatus::Pool, 10, 1),
        staff_task("T-plan", TaskStatus::Planning, 20, 2),
        staff_task("T-run", TaskStatus::Running, 30, 3),
    ];

    let projection = super::build_staff_mode_projection(
        &tasks,
        &config,
        &["external-running".to_string()],
        "dev_lifecycle",
    )
    .expect("dev lifecycle projection");

    let pool_assignment = projection
        .lanes
        .iter()
        .find(|lane| lane.status == TaskStatus::Pool)
        .and_then(|lane| lane.assignments.iter().find(|assignment| assignment.task_id == "T-pool"))
        .expect("pool task assignment");
    assert_eq!(pool_assignment.member_id.as_deref(), Some("orch-free"));
    assert!(!pool_assignment.blocked);
    assert!(pool_assignment.actions.can_start);

    let planning_assignment = projection
        .unassigned
        .iter()
        .find(|assignment| assignment.task_id == "T-plan")
        .expect("planning task unassigned");
    assert_eq!(planning_assignment.role, Some(super::TaskBoardStaffRole::Planner));
    assert!(planning_assignment.block_reason.as_deref().expect("block reason").contains("planner"));

    let executor_load = projection
        .member_loads
        .iter()
        .find(|load| load.member_id == "executor")
        .expect("executor load");
    assert_eq!(executor_load.task_ids, vec!["T-run".to_string()]);
    assert!(executor_load.running_task_ids.is_empty());
}

#[test]
fn staff_projection_requires_loaded_gateway_lifecycle() {
    let config = super::TaskBoardStaffConfig::default();
    let task = staff_task("T-missing-lifecycle", TaskStatus::Pending, 10, 1);

    let error =
        super::build_staff_mode_projection(&[task], &config, &[], "missing_gateway_lifecycle")
            .expect_err("unknown lifecycle should fail projection");

    assert!(
        error.contains("missing_gateway_lifecycle"),
        "error should include workflow id: {error}"
    );
}

#[test]
fn staff_projection_rejects_unknown_workflow_id() {
    let error = super::build_staff_mode_projection(
        &[],
        &super::TaskBoardStaffConfig::default(),
        &[],
        "missing_staff_workflow",
    )
    .expect_err("unknown workflow must be rejected");

    assert_eq!(error, "unknown task board workflow id: missing_staff_workflow");
}

#[test]
fn routing_marks_blocked_tasks_and_keeps_display_only_tasks_read_only() {
    let mut config = super::TaskBoardStaffConfig::default();
    let executor = config
        .members
        .iter_mut()
        .find(|member| member.role == super::TaskBoardStaffRole::Executor)
        .expect("executor member");
    executor.current_task_ids = vec!["T-failed".to_string()];
    let reviewer = config
        .members
        .iter_mut()
        .find(|member| member.role == super::TaskBoardStaffRole::Reviewer)
        .expect("reviewer member");
    reviewer.current_task_ids = vec!["T-paused".to_string()];

    let mut failed = staff_task("T-failed", TaskStatus::Failed, 10, 1);
    failed.last_error = Some("build failed".to_string());
    let mut paused = staff_task("T-paused", TaskStatus::Paused, 20, 2);
    paused.pause_reason = Some("waiting for input".to_string());
    let completed = staff_task("T-done", TaskStatus::Completed, 30, 3);

    let projection = super::build_staff_mode_projection(
        &[failed, paused, completed],
        &config,
        &["T-failed".to_string()],
        "dev_lifecycle",
    )
    .expect("dev lifecycle projection");

    let failed_assignment = projection
        .lanes
        .iter()
        .find(|lane| lane.status == TaskStatus::Failed)
        .and_then(|lane| {
            lane.assignments.iter().find(|assignment| assignment.task_id == "T-failed")
        })
        .expect("failed task assignment");
    assert_eq!(failed_assignment.member_id.as_deref(), Some("staff-executor"));
    assert!(failed_assignment.blocked);
    assert!(
        failed_assignment.block_reason.as_deref().expect("failed reason").contains("build failed")
    );

    let paused_assignment = projection
        .lanes
        .iter()
        .find(|lane| lane.status == TaskStatus::Paused)
        .and_then(|lane| {
            lane.assignments.iter().find(|assignment| assignment.task_id == "T-paused")
        })
        .expect("paused task assignment");
    assert_eq!(paused_assignment.member_id.as_deref(), Some("staff-reviewer"));
    assert!(paused_assignment.blocked);

    let completed_assignment = projection
        .lanes
        .iter()
        .find(|lane| lane.status == TaskStatus::Completed)
        .and_then(|lane| lane.assignments.iter().find(|assignment| assignment.task_id == "T-done"))
        .expect("completed task assignment");
    assert!(completed_assignment.display_only);
    assert_eq!(completed_assignment.member_id, None);
    assert!(!completed_assignment.actions.can_assign);
    assert!(completed_assignment.actions.can_open_logs);
}

#[test]
fn routing_uses_stage_primary_and_backup_members() {
    let mut config = super::TaskBoardStaffConfig::default();
    config.members = vec![
        super::TaskBoardStaffMember::new(
            "review-primary",
            "Review Primary",
            super::TaskBoardStaffRole::Reviewer,
        ),
        super::TaskBoardStaffMember::new(
            "review-backup",
            "Review Backup",
            super::TaskBoardStaffRole::Reviewer,
        ),
        super::TaskBoardStaffMember::new(
            "review-fallback",
            "Review Fallback",
            super::TaskBoardStaffRole::Reviewer,
        ),
    ];
    config.members[0].capacity = 1;
    config.stage_assignments = vec![super::TaskBoardStageAssignment {
        status: TaskStatus::CodeReview,
        primary_member_id: Some("review-primary".to_string()),
        backup_member_ids: vec!["review-backup".to_string()],
        manager_member_ids: Vec::new(),
        autonomous: true,
    }];

    let first = staff_task("T-review-1", TaskStatus::CodeReview, 10, 1);
    let second = staff_task("T-review-2", TaskStatus::CodeReview, 20, 2);
    let projection =
        super::build_staff_mode_projection(&[first, second], &config, &[], "dev_lifecycle")
            .expect("dev lifecycle projection");

    let review_lane = projection
        .lanes
        .iter()
        .find(|lane| lane.status == TaskStatus::CodeReview)
        .expect("review lane");
    let first_assignment = review_lane
        .assignments
        .iter()
        .find(|assignment| assignment.task_id == "T-review-1")
        .expect("first review assignment");
    let second_assignment = review_lane
        .assignments
        .iter()
        .find(|assignment| assignment.task_id == "T-review-2")
        .expect("second review assignment");

    assert_eq!(first_assignment.member_id.as_deref(), Some("review-primary"));
    assert_eq!(second_assignment.member_id.as_deref(), Some("review-backup"));
    assert!(projection.unassigned.is_empty());
}

#[test]
fn routing_skips_disabled_stage_primary_before_backup() {
    let mut config = super::TaskBoardStaffConfig::default();
    config.members = vec![
        super::TaskBoardStaffMember::new(
            "executor-primary",
            "Executor Primary",
            super::TaskBoardStaffRole::Executor,
        ),
        super::TaskBoardStaffMember::new(
            "executor-backup",
            "Executor Backup",
            super::TaskBoardStaffRole::Executor,
        ),
    ];
    config.members[0].enabled = false;
    config.stage_assignments = vec![super::TaskBoardStageAssignment {
        status: TaskStatus::Running,
        primary_member_id: Some("executor-primary".to_string()),
        backup_member_ids: vec!["executor-backup".to_string()],
        manager_member_ids: Vec::new(),
        autonomous: false,
    }];

    let projection = super::build_staff_mode_projection(
        &[staff_task("T-run", TaskStatus::Running, 10, 1)],
        &config,
        &[],
        "dev_lifecycle",
    )
    .expect("dev lifecycle projection");

    let assignment = projection
        .lanes
        .iter()
        .find(|lane| lane.status == TaskStatus::Running)
        .and_then(|lane| lane.assignments.iter().find(|assignment| assignment.task_id == "T-run"))
        .expect("running assignment");

    assert_eq!(assignment.member_id.as_deref(), Some("executor-backup"));
    assert!(projection.unassigned.is_empty());
}

#[test]
fn routing_reports_stage_assignment_without_candidates_and_missing_role() {
    let mut config = super::TaskBoardStaffConfig::default();
    let pool_stage = config
        .workflow
        .iter_mut()
        .find(|stage| stage.status == TaskStatus::Pool)
        .expect("pool stage");
    pool_stage.role = None;
    config.stage_assignments = vec![super::TaskBoardStageAssignment {
        status: TaskStatus::Pool,
        primary_member_id: None,
        backup_member_ids: vec!["  ".to_string()],
        manager_member_ids: Vec::new(),
        autonomous: false,
    }];

    let projection = super::build_staff_mode_projection(
        &[staff_task("T-unrouted", TaskStatus::Pool, 10, 1)],
        &config,
        &[],
        "dev_lifecycle",
    )
    .expect("projection");

    let assignment = projection
        .unassigned
        .iter()
        .find(|assignment| assignment.task_id == "T-unrouted")
        .expect("unassigned task");
    let reason = assignment.block_reason.as_deref().expect("block reason");
    assert!(reason.contains("no primary or backup staff member configured"));
    assert!(reason.contains("no staff role configured"));
    assert!(assignment.blocked);
    assert!(!assignment.actions.can_assign);
}

#[test]
fn routing_reports_missing_and_full_stage_assignment_members() {
    let mut config = super::TaskBoardStaffConfig::default();
    config.members = vec![super::TaskBoardStaffMember::new(
        "review-full",
        "Review Full",
        super::TaskBoardStaffRole::Reviewer,
    )];
    config.members[0].capacity = 1;
    config.members[0].current_task_ids = vec!["external-review".to_string()];
    config.stage_assignments = vec![super::TaskBoardStageAssignment {
        status: TaskStatus::CodeReview,
        primary_member_id: Some("missing-reviewer".to_string()),
        backup_member_ids: vec!["review-full".to_string()],
        manager_member_ids: Vec::new(),
        autonomous: false,
    }];

    let projection = super::build_staff_mode_projection(
        &[staff_task("T-review", TaskStatus::CodeReview, 10, 1)],
        &config,
        &[],
        "dev_lifecycle",
    )
    .expect("projection");

    let assignment = projection
        .unassigned
        .iter()
        .find(|assignment| assignment.task_id == "T-review")
        .expect("unassigned review");
    let reason = assignment.block_reason.as_deref().expect("block reason");
    assert!(reason.contains("configured member missing-reviewer does not exist"));
    assert!(reason.contains("configured member review-full has no available capacity"));
}

#[test]
fn routing_prefers_original_assignment_hints_and_records_member_metadata() {
    let mut config = super::TaskBoardStaffConfig::default();
    let executor =
        config.members.iter_mut().find(|member| member.id == "staff-executor").expect("executor");
    executor.capacity = 2;
    executor.acp_agent = Some("agent-executor".to_string());
    executor.delegate_agent = Some("delegate-executor".to_string());
    executor.executor_backend = Some(crate::app::task::TaskExecutorBackend::Codex);

    let mut by_name = staff_task("T-by-name", TaskStatus::Running, 10, 1);
    by_name.assignee = "Executor".to_string();
    let mut by_agent = staff_task("T-by-agent", TaskStatus::Running, 20, 2);
    by_agent.acp_agent = Some(" agent-executor ".to_string());

    let projection = super::build_staff_mode_projection(
        &[by_name, by_agent],
        &config,
        &["T-by-agent".to_string()],
        "dev_lifecycle",
    )
    .expect("projection");

    let running_lane = projection
        .lanes
        .iter()
        .find(|lane| lane.status == TaskStatus::Running)
        .expect("running lane");
    assert_eq!(
        running_lane
            .assignments
            .iter()
            .map(|assignment| assignment.member_id.as_deref())
            .collect::<Vec<_>>(),
        vec![Some("staff-executor"), Some("staff-executor")]
    );

    let load = projection
        .member_loads
        .iter()
        .find(|load| load.member_id == "staff-executor")
        .expect("executor load");
    assert_eq!(load.delegate_agent.as_deref(), Some("delegate-executor"));
    assert_eq!(load.acp_agent.as_deref(), Some("agent-executor"));
    assert_eq!(load.executor_backend, Some(crate::app::task::TaskExecutorBackend::Codex));
    assert_eq!(load.running_task_ids, vec!["T-by-agent".to_string()]);
}

#[test]
fn routing_marks_blocked_tasks_without_original_member_and_over_capacity_loads() {
    let mut config = super::TaskBoardStaffConfig::default();
    let executor =
        config.members.iter_mut().find(|member| member.id == "staff-executor").expect("executor");
    executor.capacity = 1;
    executor.current_task_ids = vec!["external-a".to_string(), "external-b".to_string()];

    let failed = staff_task("T-failed-unassigned", TaskStatus::Failed, 10, 1);
    let paused = staff_task("T-paused-unassigned", TaskStatus::Paused, 11, 2);

    let projection = super::build_staff_mode_projection(
        &[failed, paused],
        &config,
        &["external-a".to_string()],
        "dev_lifecycle",
    )
    .expect("projection");

    assert_eq!(projection.blocked_task_count, 2);
    for task_id in ["T-failed-unassigned", "T-paused-unassigned"] {
        let assignment = projection
            .lanes
            .iter()
            .flat_map(|lane| lane.assignments.iter())
            .find(|assignment| assignment.task_id == task_id)
            .expect("blocked assignment");
        assert!(assignment.blocked);
        assert!(
            assignment
                .block_reason
                .as_deref()
                .expect("block reason")
                .contains("original staff member is not assigned")
        );
        assert!(!assignment.actions.can_start);
    }

    let load = projection
        .member_loads
        .iter()
        .find(|load| load.member_id == "staff-executor")
        .expect("executor load");
    assert_eq!(load.task_ids, vec!["external-a".to_string(), "external-b".to_string()]);
    assert_eq!(load.running_task_ids, vec!["external-a".to_string()]);
    assert!(load.over_capacity);
}

#[test]
fn staff_custom_workflow_stage_actions_follow_status_roles() {
    let mut config = super::TaskBoardStaffConfig::default()
        .retarget_for_workflow_id("marketing_campaign")
        .expect("marketing staff workflow");
    config.stage_actions = super::models::default_stage_actions_for_workflow(
        &crate::app::task::workflow::task_workflow_by_id("marketing_campaign")
            .expect("marketing workflow"),
    );
    for member in &mut config.members {
        match member.id.as_str() {
            "staff-orchestrator" => member.current_task_ids = vec!["T-campaign-queued".to_string()],
            "staff-planner" => member.current_task_ids = vec!["T-campaign-planning".to_string()],
            "staff-executor" => member.current_task_ids = vec!["T-campaign-running".to_string()],
            "staff-reviewer" => {
                member.capacity = 2;
                member.current_task_ids =
                    vec!["T-campaign-review-ready".to_string(), "T-campaign-reviewing".to_string()];
            }
            "staff-merger" => member.current_task_ids = vec!["T-campaign-closing".to_string()],
            _ => {}
        }
    }
    let campaign_queued = TaskStatus::Custom("campaign_queued");
    let campaign_planning = TaskStatus::Custom("campaign_planning");
    let campaign_running = TaskStatus::Custom("campaign_running");
    let campaign_review_ready = TaskStatus::Custom("campaign_review_ready");
    let campaign_reviewing = TaskStatus::Custom("campaign_reviewing");
    let campaign_closing = TaskStatus::Custom("campaign_closing");
    let campaign_done = TaskStatus::Custom("campaign_done");
    let campaign_archived = TaskStatus::Custom("campaign_archived");
    let tasks = vec![
        staff_task("T-campaign-queued", campaign_queued, 10, 1),
        staff_task("T-campaign-planning", campaign_planning, 11, 2),
        staff_task("T-campaign-running", campaign_running, 12, 3),
        staff_task("T-campaign-review-ready", campaign_review_ready, 13, 4),
        staff_task("T-campaign-reviewing", campaign_reviewing, 14, 5),
        staff_task("T-campaign-closing", campaign_closing, 15, 6),
        staff_task("T-campaign-done", campaign_done, 16, 7),
        staff_task("T-campaign-archived", campaign_archived, 17, 8),
    ];

    let projection = super::build_staff_mode_projection(&tasks, &config, &[], "marketing_campaign")
        .expect("marketing lifecycle projection");

    assert_stage_start_action(
        &projection,
        campaign_queued,
        "T-campaign-queued",
        Some("staff-orchestrator"),
        true,
    );
    assert_stage_start_action(
        &projection,
        campaign_planning,
        "T-campaign-planning",
        Some("staff-planner"),
        true,
    );
    assert_stage_start_action(
        &projection,
        campaign_running,
        "T-campaign-running",
        Some("staff-executor"),
        false,
    );
    assert_stage_start_action(
        &projection,
        campaign_review_ready,
        "T-campaign-review-ready",
        Some("staff-reviewer"),
        true,
    );
    assert_stage_start_action(
        &projection,
        campaign_reviewing,
        "T-campaign-reviewing",
        Some("staff-reviewer"),
        true,
    );
    assert_stage_start_action(
        &projection,
        campaign_closing,
        "T-campaign-closing",
        Some("staff-merger"),
        true,
    );
    assert_stage_start_action(&projection, campaign_done, "T-campaign-done", None, false);
    assert_stage_start_action(&projection, campaign_archived, "T-campaign-archived", None, false);

    let queued_command = TaskBoardStaffCommand::StartTask {
        command_id: "cmd-campaign-queued".to_string(),
        task_id: "T-campaign-queued".to_string(),
        member_id: "staff-orchestrator".to_string(),
    };
    let queued_action =
        super::validate_staff_command(&queued_command, &tasks, &config, &[], "marketing_campaign")
            .expect("queued custom status starts execution");
    assert_eq!(
        queued_action,
        TaskBoardStaffCommandAction::ExecuteTask { task_id: "T-campaign-queued".to_string() }
    );

    let closing_command = TaskBoardStaffCommand::StartTask {
        command_id: "cmd-campaign-closing".to_string(),
        task_id: "T-campaign-closing".to_string(),
        member_id: "staff-merger".to_string(),
    };
    let closing_action =
        super::validate_staff_command(&closing_command, &tasks, &config, &[], "marketing_campaign")
            .expect("merging custom status starts merge");
    assert_eq!(
        closing_action,
        TaskBoardStaffCommandAction::ChangeTaskStatus {
            task_id: "T-campaign-closing".to_string(),
            new_status: campaign_done,
        }
    );
}

fn assert_stage_start_action(
    projection: &super::TaskBoardStaffProjection,
    status: TaskStatus,
    task_id: &str,
    expected_member_id: Option<&str>,
    expected_can_start: bool,
) {
    let assignment = projection
        .lanes
        .iter()
        .find(|lane| lane.status == status)
        .and_then(|lane| lane.assignments.iter().find(|assignment| assignment.task_id == task_id))
        .expect("stage assignment");
    assert_eq!(assignment.member_id.as_deref(), expected_member_id);
    assert_eq!(assignment.actions.can_start, expected_can_start);
}
