//! 员工自动调度测试，覆盖自主阶段、重新分配、备选成员和跳过原因。
//! 调度器仅为可执行且容量允许的任务发出唯一命令，配置错误需先于调度失败。

use std::collections::HashSet;

use crate::app::task::{Task, TaskBoardStaffCommand, TaskStatus};

fn staff_task(id: &str, status: TaskStatus, priority: u32, order: u32) -> Task {
    let mut task = Task::default();
    task.id = id.to_string();
    task.status = status;
    task.priority = priority;
    task.order = order;
    task.description = format!("Task {id}");
    task
}

fn skip_reason<'a>(plan: &'a super::TaskBoardStaffAutoSchedulePlan, task_id: &str) -> &'a str {
    plan.skipped
        .iter()
        .find(|skip| skip.task_id == task_id)
        .map(|skip| skip.reason.as_str())
        .expect("skip reason")
}

#[test]
fn auto_scheduler_rejects_unknown_workflow_id_before_scheduling() {
    let error = super::build_staff_auto_schedule_plan(
        &[],
        &super::TaskBoardStaffConfig::default(),
        &super::TaskBoardStaffProjection::default(),
        &[],
        "missing_staff_workflow",
        42,
    )
    .expect_err("unknown workflow must be rejected");

    assert_eq!(error, "unknown task board workflow id: missing_staff_workflow");
}

#[test]
fn auto_scheduler_emits_commands_only_for_autonomous_stage_assignments() {
    let mut config = super::TaskBoardStaffConfig::default();
    config.members = vec![
        super::TaskBoardStaffMember::new(
            "staff-orchestrator",
            "Orchestrator",
            super::TaskBoardStaffRole::Orchestrator,
        ),
        super::TaskBoardStaffMember::new(
            "staff-planner",
            "Planner",
            super::TaskBoardStaffRole::Planner,
        ),
        super::TaskBoardStaffMember::new(
            "staff-executor",
            "Executor",
            super::TaskBoardStaffRole::Executor,
        ),
        super::TaskBoardStaffMember::new(
            "reviewer-full",
            "Reviewer Full",
            super::TaskBoardStaffRole::Reviewer,
        ),
        super::TaskBoardStaffMember::new(
            "reviewer-disabled",
            "Reviewer Disabled",
            super::TaskBoardStaffRole::Reviewer,
        ),
        super::TaskBoardStaffMember::new(
            "reviewer-wrong-role",
            "Reviewer Wrong Role",
            super::TaskBoardStaffRole::Reviewer,
        ),
    ];
    config.members[0].current_task_ids =
        vec!["T-paused-resume".to_string(), "T-pending-noop".to_string()];
    config.members[1].current_task_ids = vec!["T-planning-start".to_string()];
    config.members[2].capacity = 2;
    config.members[2].current_task_ids = vec!["T-running-claimed".to_string()];
    config.members[3].capacity = 1;
    config.members[3].current_task_ids = vec!["external-review".to_string()];
    config.members[4].enabled = false;
    config.stage_assignments = vec![
        super::TaskBoardStageAssignment {
            status: TaskStatus::Running,
            primary_member_id: Some("staff-executor".to_string()),
            backup_member_ids: Vec::new(),
            manager_member_ids: Vec::new(),
            autonomous: true,
        },
        super::TaskBoardStageAssignment {
            status: TaskStatus::Planning,
            primary_member_id: Some("staff-planner".to_string()),
            backup_member_ids: Vec::new(),
            manager_member_ids: Vec::new(),
            autonomous: true,
        },
        super::TaskBoardStageAssignment {
            status: TaskStatus::Paused,
            primary_member_id: Some("staff-orchestrator".to_string()),
            backup_member_ids: Vec::new(),
            manager_member_ids: Vec::new(),
            autonomous: true,
        },
        super::TaskBoardStageAssignment {
            status: TaskStatus::Pending,
            primary_member_id: Some("staff-orchestrator".to_string()),
            backup_member_ids: Vec::new(),
            manager_member_ids: Vec::new(),
            autonomous: false,
        },
        super::TaskBoardStageAssignment {
            status: TaskStatus::CodeReview,
            primary_member_id: Some("reviewer-full".to_string()),
            backup_member_ids: Vec::new(),
            manager_member_ids: Vec::new(),
            autonomous: true,
        },
        super::TaskBoardStageAssignment {
            status: TaskStatus::CodeComplete,
            primary_member_id: Some("reviewer-disabled".to_string()),
            backup_member_ids: Vec::new(),
            manager_member_ids: Vec::new(),
            autonomous: true,
        },
        super::TaskBoardStageAssignment {
            status: TaskStatus::PrSubmitted,
            primary_member_id: Some("reviewer-wrong-role".to_string()),
            backup_member_ids: Vec::new(),
            manager_member_ids: Vec::new(),
            autonomous: true,
        },
        super::TaskBoardStageAssignment {
            status: TaskStatus::Completed,
            primary_member_id: Some("staff-orchestrator".to_string()),
            backup_member_ids: Vec::new(),
            manager_member_ids: Vec::new(),
            autonomous: true,
        },
    ];
    let tasks = vec![
        staff_task("T-running-new", TaskStatus::Running, 10, 1),
        staff_task("T-running-claimed", TaskStatus::Running, 11, 2),
        staff_task("T-planning-start", TaskStatus::Planning, 12, 3),
        staff_task("T-paused-resume", TaskStatus::Paused, 13, 4),
        staff_task("T-pending-noop", TaskStatus::Pending, 14, 5),
        staff_task("T-review-capacity", TaskStatus::CodeReview, 15, 6),
        staff_task("T-code-disabled", TaskStatus::CodeComplete, 16, 7),
        staff_task("T-merge-role", TaskStatus::PrSubmitted, 17, 8),
        staff_task("T-completed", TaskStatus::Completed, 18, 9),
    ];
    let running_task_ids = vec!["T-running-claimed".to_string()];
    let projection =
        super::build_staff_mode_projection(&tasks, &config, &running_task_ids, "dev_lifecycle")
            .expect("dev lifecycle projection");

    let plan = super::build_staff_auto_schedule_plan(
        &tasks,
        &config,
        &projection,
        &running_task_ids,
        "dev_lifecycle",
        42,
    )
    .expect("dev lifecycle auto schedule");

    assert!(plan.commands.iter().any(|command| {
        matches!(
            command,
            TaskBoardStaffCommand::AssignTask { task_id, member_id, .. }
                if task_id == "T-running-new" && member_id == "staff-executor"
        )
    }));
    assert!(plan.commands.iter().any(|command| {
        matches!(
            command,
            TaskBoardStaffCommand::StartTask { task_id, member_id, .. }
                if task_id == "T-planning-start" && member_id == "staff-planner"
        )
    }));
    assert!(plan.commands.iter().any(|command| {
        matches!(
            command,
            TaskBoardStaffCommand::ResumeTask { task_id, member_id, .. }
                if task_id == "T-paused-resume" && member_id == "staff-orchestrator"
        )
    }));
    assert_eq!(plan.commands.len(), 3);
    assert_eq!(plan.command_contexts.len(), 3);
    assert!(plan.command_contexts.iter().any(|context| {
        context.orchestration.stage_status == TaskStatus::Running
            && context.orchestration.member_level_label == "执行智能体"
            && context.orchestration.target.contains("T-running-new")
    }));
    assert!(plan.command_contexts.iter().any(|context| {
        context.orchestration.stage_status == TaskStatus::Planning
            && context.orchestration.member_level_label == "管理智能体"
            && context.orchestration.target.contains("T-planning-start")
    }));
    assert!(plan.command_contexts.iter().any(|context| {
        context.orchestration.stage_status == TaskStatus::Paused
            && context.orchestration.member_level_label == "主智能体"
            && context.orchestration.target.contains("T-paused-resume")
    }));

    let command_ids =
        plan.commands.iter().map(TaskBoardStaffCommand::command_id).collect::<HashSet<_>>();
    assert_eq!(command_ids.len(), plan.commands.len());

    assert!(plan.commands.iter().all(|command| command.task_id() != Some("T-pending-noop")));
    assert!(skip_reason(&plan, "T-running-claimed").contains("running"));
    assert!(skip_reason(&plan, "T-review-capacity").contains("capacity"));
    assert!(skip_reason(&plan, "T-code-disabled").contains("disabled"));
    assert!(skip_reason(&plan, "T-merge-role").contains("role"));
    assert!(skip_reason(&plan, "T-completed").contains("ended"));
}

#[test]
fn auto_scheduler_starts_assigned_pool_task() {
    let mut config = super::TaskBoardStaffConfig::default();
    config.members = vec![super::TaskBoardStaffMember::new(
        "staff-orchestrator",
        "Orchestrator",
        super::TaskBoardStaffRole::Orchestrator,
    )];
    config.members[0].current_task_ids = vec!["T-pool".to_string()];
    config.stage_assignments = vec![super::TaskBoardStageAssignment {
        status: TaskStatus::Pool,
        primary_member_id: Some("staff-orchestrator".to_string()),
        backup_member_ids: Vec::new(),
        manager_member_ids: Vec::new(),
        autonomous: true,
    }];
    let task = staff_task("T-pool", TaskStatus::Pool, 10, 1);
    let projection =
        super::build_staff_mode_projection(&[task.clone()], &config, &[], "dev_lifecycle")
            .expect("dev lifecycle projection");

    let plan = super::build_staff_auto_schedule_plan(
        &[task],
        &config,
        &projection,
        &[],
        "dev_lifecycle",
        42,
    )
    .expect("dev lifecycle auto schedule");

    assert_eq!(plan.commands.len(), 1);
    assert!(matches!(
        &plan.commands[0],
        TaskBoardStaffCommand::StartTask { task_id, member_id, .. }
            if task_id == "T-pool" && member_id == "staff-orchestrator"
    ));
    assert!(plan.skipped.is_empty());
}

#[test]
fn auto_scheduler_reassigns_stale_member_after_stage_change() {
    let mut config = super::TaskBoardStaffConfig::default();
    let executor =
        config.members.iter_mut().find(|member| member.id == "staff-executor").expect("executor");
    executor.current_task_ids = vec!["T-code-complete".to_string()];
    config.stage_assignments = vec![super::TaskBoardStageAssignment {
        status: TaskStatus::CodeComplete,
        primary_member_id: Some("staff-reviewer".to_string()),
        backup_member_ids: Vec::new(),
        manager_member_ids: Vec::new(),
        autonomous: true,
    }];
    let task = staff_task("T-code-complete", TaskStatus::CodeComplete, 10, 1);
    let projection =
        super::build_staff_mode_projection(&[task.clone()], &config, &[], "dev_lifecycle")
            .expect("dev lifecycle projection");

    let plan = super::build_staff_auto_schedule_plan(
        &[task],
        &config,
        &projection,
        &[],
        "dev_lifecycle",
        42,
    )
    .expect("dev lifecycle auto schedule");

    assert_eq!(plan.commands.len(), 1);
    assert!(matches!(
        &plan.commands[0],
        TaskBoardStaffCommand::AssignTask { task_id, member_id, .. }
            if task_id == "T-code-complete" && member_id == "staff-reviewer"
    ));
    assert!(plan.skipped.is_empty());
}

#[test]
fn auto_scheduler_uses_first_available_unique_backup_member() {
    let mut config = super::TaskBoardStaffConfig::default();
    config.members = vec![
        super::TaskBoardStaffMember::new(
            "disabled-executor",
            "Disabled Executor",
            super::TaskBoardStaffRole::Executor,
        ),
        super::TaskBoardStaffMember::new(
            "backup-executor",
            "Backup Executor",
            super::TaskBoardStaffRole::Executor,
        ),
    ];
    config.members[0].enabled = false;
    config.stage_assignments = vec![super::TaskBoardStageAssignment {
        status: TaskStatus::Running,
        primary_member_id: Some("missing-executor".to_string()),
        backup_member_ids: vec![
            " ".to_string(),
            "disabled-executor".to_string(),
            "backup-executor".to_string(),
            "backup-executor".to_string(),
        ],
        manager_member_ids: Vec::new(),
        autonomous: true,
    }];
    let task = staff_task("T-backup", TaskStatus::Running, 10, 1);
    let projection =
        super::build_staff_mode_projection(&[task.clone()], &config, &[], "dev_lifecycle")
            .expect("projection should build");

    let plan = super::build_staff_auto_schedule_plan(
        &[task],
        &config,
        &projection,
        &[],
        "dev_lifecycle",
        42,
    )
    .expect("plan should build");

    assert_eq!(plan.commands.len(), 1);
    assert!(matches!(
        &plan.commands[0],
        TaskBoardStaffCommand::AssignTask { task_id, member_id, .. }
            if task_id == "T-backup" && member_id == "backup-executor"
    ));
    assert!(plan.skipped.is_empty());
}

#[test]
fn auto_scheduler_reports_empty_candidates_and_missing_role_configuration() {
    let mut no_candidate_config = super::TaskBoardStaffConfig::default();
    no_candidate_config.stage_assignments = vec![super::TaskBoardStageAssignment {
        status: TaskStatus::Running,
        primary_member_id: Some(" ".to_string()),
        backup_member_ids: vec!["".to_string()],
        manager_member_ids: Vec::new(),
        autonomous: true,
    }];
    let running = staff_task("T-no-candidates", TaskStatus::Running, 10, 1);
    let projection = super::build_staff_mode_projection(
        &[running.clone()],
        &no_candidate_config,
        &[],
        "dev_lifecycle",
    )
    .expect("projection should build");

    let plan = super::build_staff_auto_schedule_plan(
        &[running],
        &no_candidate_config,
        &projection,
        &[],
        "dev_lifecycle",
        42,
    )
    .expect("plan should build");

    assert_eq!(
        skip_reason(&plan, "T-no-candidates"),
        "no primary or backup staff member configured for autonomous stage running"
    );

    let custom_status = TaskStatus::Custom("unmapped_stage");
    let mut missing_role_config = super::TaskBoardStaffConfig::default();
    missing_role_config.stage_assignments = vec![super::TaskBoardStageAssignment {
        status: custom_status,
        primary_member_id: Some("staff-executor".to_string()),
        backup_member_ids: Vec::new(),
        manager_member_ids: Vec::new(),
        autonomous: true,
    }];
    let custom = staff_task("T-missing-role", custom_status, 10, 1);
    let error = super::build_staff_mode_projection(&[custom], &missing_role_config, &[], "")
        .expect_err("unmapped custom stage should be rejected");

    assert!(error.contains("does not define status unmapped_stage"));
}

#[test]
fn push_command_records_duplicate_auto_schedule_command_as_skip() {
    let mut plan = super::TaskBoardStaffAutoSchedulePlan::default();
    let mut scheduled = HashSet::new();
    let command = TaskBoardStaffCommand::AssignTask {
        command_id: "cmd-1".to_string(),
        task_id: "T-dup".to_string(),
        member_id: "staff-executor".to_string(),
    };

    assert!(super::push_command(
        &mut plan,
        &mut scheduled,
        command.clone(),
        TaskStatus::Running,
        None,
    ));
    assert!(!super::push_command(&mut plan, &mut scheduled, command, TaskStatus::Running, None,));

    assert_eq!(plan.commands.len(), 1);
    assert_eq!(plan.skipped.len(), 1);
    assert_eq!(plan.skipped[0].task_id, "T-dup");
    assert_eq!(plan.skipped[0].reason, "duplicate auto schedule command");
}

#[test]
fn auto_scheduler_handles_marketing_and_recruiting_statuses() {
    let campaign_queued = TaskStatus::Custom("campaign_queued");
    let campaign_done = TaskStatus::Custom("campaign_done");
    let campaign_archived = TaskStatus::Custom("campaign_archived");
    let mut marketing_config = super::TaskBoardStaffConfig::default()
        .retarget_for_workflow_id("marketing_campaign")
        .expect("marketing staff workflow");
    let orchestrator = marketing_config
        .members
        .iter_mut()
        .find(|member| member.id == "staff-orchestrator")
        .expect("orchestrator");
    orchestrator.current_task_ids = vec!["T-campaign-queued".to_string()];
    marketing_config.stage_assignments = vec![
        super::TaskBoardStageAssignment {
            status: campaign_queued,
            primary_member_id: Some("staff-orchestrator".to_string()),
            backup_member_ids: Vec::new(),
            manager_member_ids: Vec::new(),
            autonomous: true,
        },
        super::TaskBoardStageAssignment {
            status: campaign_done,
            primary_member_id: Some("staff-orchestrator".to_string()),
            backup_member_ids: Vec::new(),
            manager_member_ids: Vec::new(),
            autonomous: true,
        },
        super::TaskBoardStageAssignment {
            status: campaign_archived,
            primary_member_id: Some("staff-orchestrator".to_string()),
            backup_member_ids: Vec::new(),
            manager_member_ids: Vec::new(),
            autonomous: true,
        },
    ];
    let marketing_tasks = vec![
        staff_task("T-campaign-queued", campaign_queued, 10, 1),
        staff_task("T-campaign-done", campaign_done, 11, 2),
        staff_task("T-campaign-archived", campaign_archived, 12, 3),
    ];
    let marketing_projection = super::build_staff_mode_projection(
        &marketing_tasks,
        &marketing_config,
        &[],
        "marketing_campaign",
    )
    .expect("marketing lifecycle projection");

    let marketing_plan = super::build_staff_auto_schedule_plan(
        &marketing_tasks,
        &marketing_config,
        &marketing_projection,
        &[],
        "marketing_campaign",
        42,
    )
    .expect("marketing lifecycle auto schedule");

    assert!(marketing_plan.commands.iter().any(|command| {
        matches!(
            command,
            TaskBoardStaffCommand::StartTask { task_id, member_id, .. }
                if task_id == "T-campaign-queued" && member_id == "staff-orchestrator"
        )
    }));
    assert!(
        marketing_plan.commands.iter().all(|command| command.task_id() != Some("T-campaign-done"))
    );
    assert!(
        marketing_plan
            .commands
            .iter()
            .all(|command| command.task_id() != Some("T-campaign-archived"))
    );

    let recruiting_screening = TaskStatus::Custom("recruiting_screening");
    let mut recruiting_config = super::TaskBoardStaffConfig::default()
        .retarget_for_workflow_id("recruiting_pipeline")
        .expect("recruiting staff workflow");
    let planner = recruiting_config
        .members
        .iter_mut()
        .find(|member| member.id == "staff-planner")
        .expect("planner");
    planner.capacity = 3;
    planner.current_task_ids = vec!["T-recruiting-screening-start".to_string()];
    recruiting_config.stage_assignments = vec![super::TaskBoardStageAssignment {
        status: recruiting_screening,
        primary_member_id: Some("staff-planner".to_string()),
        backup_member_ids: Vec::new(),
        manager_member_ids: Vec::new(),
        autonomous: true,
    }];
    let recruiting_tasks = vec![
        staff_task("T-recruiting-screening-start", recruiting_screening, 20, 1),
        staff_task("T-recruiting-screening-assign", recruiting_screening, 21, 2),
    ];
    let recruiting_projection = super::build_staff_mode_projection(
        &recruiting_tasks,
        &recruiting_config,
        &[],
        "recruiting_pipeline",
    )
    .expect("recruiting lifecycle projection");

    let recruiting_plan = super::build_staff_auto_schedule_plan(
        &recruiting_tasks,
        &recruiting_config,
        &recruiting_projection,
        &[],
        "recruiting_pipeline",
        43,
    )
    .expect("recruiting lifecycle auto schedule");

    assert!(recruiting_plan.commands.iter().any(|command| {
        matches!(
            command,
            TaskBoardStaffCommand::StartTask { task_id, member_id, .. }
                if task_id == "T-recruiting-screening-start" && member_id == "staff-planner"
        )
    }));
    assert!(recruiting_plan.commands.iter().any(|command| {
        matches!(
            command,
            TaskBoardStaffCommand::AssignTask { task_id, member_id, .. }
                if task_id == "T-recruiting-screening-assign" && member_id == "staff-planner"
        )
    }));
}
