use crate::app::task::{Task, TaskStatus};

use super::models::TaskBoardStageAction;
use super::{
    TaskBoardStaffCommand, TaskBoardStaffCommandAction, TaskBoardStaffConfig, TaskBoardStaffMember,
    TaskBoardStaffRole, validate_staff_command,
};

fn task_with_status(id: &str, status: TaskStatus) -> Task {
    let mut task = Task::default();
    task.id = id.to_string();
    task.status = status;
    task
}

#[test]
fn staff_command_policy_rejects_wrong_role_transition() {
    let mut config = TaskBoardStaffConfig::default();
    config.members =
        vec![TaskBoardStaffMember::new("staff-reviewer", "Reviewer", TaskBoardStaffRole::Reviewer)];
    let task = task_with_status("T-running", TaskStatus::Running);
    let command = TaskBoardStaffCommand::StartTask {
        command_id: "cmd-start".to_string(),
        task_id: task.id.clone(),
        member_id: "staff-reviewer".to_string(),
    };

    let error = validate_staff_command(&command, &[task], &config, &[], "dev_lifecycle")
        .expect_err("wrong role rejected");

    assert!(error.message.contains("角色不匹配"));
}

#[test]
fn staff_command_policy_maps_matching_start_to_existing_task_message() {
    let mut config = TaskBoardStaffConfig::default();
    config.members = vec![TaskBoardStaffMember::new(
        "staff-orchestrator",
        "Orchestrator",
        TaskBoardStaffRole::Orchestrator,
    )];
    let task = task_with_status("T-pending", TaskStatus::Pending);
    let command = TaskBoardStaffCommand::StartTask {
        command_id: "cmd-start".to_string(),
        task_id: task.id.clone(),
        member_id: "staff-orchestrator".to_string(),
    };

    let action = validate_staff_command(&command, &[task], &config, &[], "dev_lifecycle")
        .expect("matching role allowed");

    assert_eq!(
        action,
        TaskBoardStaffCommandAction::ExecuteTask { task_id: "T-pending".to_string() }
    );
}

#[test]
fn staff_command_policy_routes_late_stages_to_pipeline_ticks() {
    let mut config = TaskBoardStaffConfig::default();
    config.stage_actions = vec![
        TaskBoardStageAction {
            status: TaskStatus::CodeComplete,
            capability_id: "code_review".to_string(),
            ..Default::default()
        },
        TaskBoardStageAction {
            status: TaskStatus::CodeReview,
            capability_id: "code_review".to_string(),
            ..Default::default()
        },
        TaskBoardStageAction {
            status: TaskStatus::PrSubmitted,
            capability_id: "git_merge".to_string(),
            ..Default::default()
        },
    ];
    let cases = [
        (
            "T-code-complete",
            TaskStatus::CodeComplete,
            "staff-reviewer",
            TaskBoardStaffCommandAction::RunCodeReview { task_id: "T-code-complete".to_string() },
        ),
        (
            "T-code-review",
            TaskStatus::CodeReview,
            "staff-reviewer",
            TaskBoardStaffCommandAction::RunCodeReview { task_id: "T-code-review".to_string() },
        ),
        (
            "T-pr-submitted",
            TaskStatus::PrSubmitted,
            "staff-merger",
            TaskBoardStaffCommandAction::RunMerge { task_id: "T-pr-submitted".to_string() },
        ),
    ];

    for (task_id, status, member_id, expected) in cases {
        let task = task_with_status(&task_id, status);
        let command = TaskBoardStaffCommand::StartTask {
            command_id: format!("cmd-{task_id}"),
            task_id: task_id.to_string(),
            member_id: member_id.to_string(),
        };

        let action = validate_staff_command(&command, &[task], &config, &[], "dev_lifecycle")
            .expect("matching role allowed");

        assert_eq!(action, expected);
    }
}

#[test]
fn staff_command_accessors_cover_task_member_and_global_commands() {
    let commands = [
        TaskBoardStaffCommand::AssignTask {
            command_id: "assign".into(),
            task_id: "T1".into(),
            member_id: "M1".into(),
        },
        TaskBoardStaffCommand::TakeOverTask {
            command_id: "take-over".into(),
            task_id: "T1".into(),
            member_id: "M1".into(),
        },
        TaskBoardStaffCommand::PauseTask {
            command_id: "pause".into(),
            task_id: "T1".into(),
            member_id: "M1".into(),
        },
        TaskBoardStaffCommand::ResumeTask {
            command_id: "resume".into(),
            task_id: "T1".into(),
            member_id: "M1".into(),
        },
        TaskBoardStaffCommand::StartTask {
            command_id: "start".into(),
            task_id: "T1".into(),
            member_id: "M1".into(),
        },
        TaskBoardStaffCommand::OpenTaskLogs { command_id: "logs".into(), task_id: "T1".into() },
        TaskBoardStaffCommand::FilterByMember {
            command_id: "filter".into(),
            member_id: "M1".into(),
        },
    ];
    let expected = [
        ("assign", Some("T1"), Some("M1"), "派任务"),
        ("take-over", Some("T1"), Some("M1"), "接管任务"),
        ("pause", Some("T1"), Some("M1"), "暂停任务"),
        ("resume", Some("T1"), Some("M1"), "继续任务"),
        ("start", Some("T1"), Some("M1"), "开始任务"),
        ("logs", Some("T1"), None, "打开日志"),
        ("filter", None, Some("M1"), "筛选成员"),
    ];

    for (command, (command_id, task_id, member_id, label)) in commands.iter().zip(expected) {
        assert_eq!(command.command_id(), command_id);
        assert_eq!(command.task_id(), task_id);
        assert_eq!(command.member_id(), member_id);
        assert_eq!(command.label(), label);
    }
}

#[test]
fn staff_command_policy_handles_pause_and_resume_transitions() {
    let config = TaskBoardStaffConfig::default();
    let cases = [
        (
            task_with_status("T-running", TaskStatus::Running),
            TaskBoardStaffCommand::PauseTask {
                command_id: "pause".into(),
                task_id: "T-running".into(),
                member_id: "staff-executor".into(),
            },
            TaskStatus::Paused,
        ),
        (
            task_with_status("T-paused", TaskStatus::Paused),
            TaskBoardStaffCommand::ResumeTask {
                command_id: "resume".into(),
                task_id: "T-paused".into(),
                member_id: "staff-orchestrator".into(),
            },
            TaskStatus::Pending,
        ),
    ];

    for (task, command, expected_status) in cases {
        let task_id = task.id.clone();
        let action = validate_staff_command(&command, &[task], &config, &[], "dev_lifecycle")
            .expect("transition should be allowed");
        assert_eq!(
            action,
            TaskBoardStaffCommandAction::ChangeTaskStatus { task_id, new_status: expected_status }
        );
    }
}

#[test]
fn staff_command_policy_rejects_missing_disabled_and_full_members() {
    let task = task_with_status("T-pending", TaskStatus::Pending);
    let command = TaskBoardStaffCommand::AssignTask {
        command_id: "assign".into(),
        task_id: task.id.clone(),
        member_id: "staff-orchestrator".into(),
    };

    let missing_task_error = validate_staff_command(
        &command,
        &[],
        &TaskBoardStaffConfig::default(),
        &[],
        "dev_lifecycle",
    )
    .expect_err("missing task should fail");
    assert!(missing_task_error.message.contains("任务 T-pending 不存在"));

    let mut disabled = TaskBoardStaffConfig::default();
    disabled.members[0].enabled = false;
    let disabled_error = validate_staff_command(
        &command,
        std::slice::from_ref(&task),
        &disabled,
        &[],
        "dev_lifecycle",
    )
    .expect_err("disabled member should fail");
    assert!(disabled_error.message.contains("已停用"));

    let mut full = TaskBoardStaffConfig::default();
    full.members[0].current_task_ids = vec!["other-task".into()];
    let full_error =
        validate_staff_command(&command, std::slice::from_ref(&task), &full, &[], "dev_lifecycle")
            .expect_err("full member should fail");
    assert!(full_error.message.contains("容量已满"));

    full.members[0].current_task_ids = vec![task.id.clone()];
    assert!(validate_staff_command(&command, &[task], &full, &[], "dev_lifecycle").is_ok());
}

#[test]
fn staff_command_policy_routes_logs_and_filter_without_task_execution() {
    let task = task_with_status("T-completed", TaskStatus::Completed);
    let config = TaskBoardStaffConfig::default();

    let logs = validate_staff_command(
        &TaskBoardStaffCommand::OpenTaskLogs {
            command_id: "logs".into(),
            task_id: task.id.clone(),
        },
        std::slice::from_ref(&task),
        &config,
        &[],
        "dev_lifecycle",
    )
    .expect("completed task logs remain available");
    assert_eq!(logs, TaskBoardStaffCommandAction::ViewTaskLogs { task_id: task.id.clone() });

    let filter = validate_staff_command(
        &TaskBoardStaffCommand::FilterByMember {
            command_id: "filter".into(),
            member_id: "staff-reviewer".into(),
        },
        &[task],
        &config,
        &[],
        "dev_lifecycle",
    )
    .expect("enabled member should be filterable");
    assert_eq!(
        filter,
        TaskBoardStaffCommandAction::FilterByMember { member_id: "staff-reviewer".into() }
    );
}

#[test]
fn staff_command_policy_reports_invalid_workflow_and_stage_capability_configuration() {
    let task = task_with_status("T-pending", TaskStatus::Pending);
    let command = TaskBoardStaffCommand::StartTask {
        command_id: "start".into(),
        task_id: task.id.clone(),
        member_id: "staff-orchestrator".into(),
    };
    let invalid_workflow = validate_staff_command(
        &command,
        std::slice::from_ref(&task),
        &TaskBoardStaffConfig::default(),
        &[],
        "missing-workflow",
    )
    .expect_err("unknown workflow should fail");
    assert!(invalid_workflow.message.contains("unknown task board workflow id"));

    let mut missing_uuid = TaskBoardStaffConfig::default();
    missing_uuid.stage_actions = vec![TaskBoardStageAction {
        status: TaskStatus::Pending,
        capability_id: "saved_workflow".into(),
        ..TaskBoardStageAction::default()
    }];
    let missing_uuid_error = validate_staff_command(
        &command,
        std::slice::from_ref(&task),
        &missing_uuid,
        &[],
        "dev_lifecycle",
    )
    .expect_err("saved workflow without uuid should fail");
    assert!(missing_uuid_error.message.contains("缺少 workflow_uuid"));

    let mut manual = TaskBoardStaffConfig::default();
    manual.stage_actions = vec![TaskBoardStageAction {
        status: TaskStatus::Pending,
        capability_id: "manual".into(),
        ..TaskBoardStageAction::default()
    }];
    let manual_error = validate_staff_command(&command, &[task], &manual, &[], "dev_lifecycle")
        .expect_err("manual stage should reject automatic start");
    assert!(manual_error.message.contains("人工处理"));
}

#[test]
fn staff_command_policy_runs_saved_workflow_stage_action_with_member_context() {
    let task = task_with_status("T-pending", TaskStatus::Pending);
    let command = TaskBoardStaffCommand::StartTask {
        command_id: "start".into(),
        task_id: task.id.clone(),
        member_id: "staff-orchestrator".into(),
    };
    let mut config = TaskBoardStaffConfig::default();
    config.stage_actions = vec![TaskBoardStageAction {
        status: TaskStatus::Pending,
        capability_id: " saved_workflow ".into(),
        workflow_uuid: Some(" workflow-123 ".into()),
        ..TaskBoardStageAction::default()
    }];

    let action = validate_staff_command(&command, &[task], &config, &[], "dev_lifecycle")
        .expect("saved workflow stage action should be allowed");

    assert_eq!(
        action,
        TaskBoardStaffCommandAction::RunStageWorkflow {
            task_id: "T-pending".into(),
            status: TaskStatus::Pending,
            workflow_uuid: "workflow-123".into(),
            member_id: "staff-orchestrator".into(),
        }
    );
}

#[test]
fn staff_command_policy_rejects_finished_tasks_and_unmapped_custom_statuses() {
    let config = TaskBoardStaffConfig::default();
    let completed = task_with_status("T-completed", TaskStatus::Completed);
    let completed_command = TaskBoardStaffCommand::StartTask {
        command_id: "start".into(),
        task_id: completed.id.clone(),
        member_id: "staff-orchestrator".into(),
    };
    let completed_error =
        validate_staff_command(&completed_command, &[completed], &config, &[], "dev_lifecycle")
            .expect_err("completed tasks should reject staff execution");
    assert!(completed_error.message.contains("已结束"));

    let custom = task_with_status("T-custom", TaskStatus::Custom("unmapped_stage"));
    let custom_command = TaskBoardStaffCommand::StartTask {
        command_id: "start".into(),
        task_id: custom.id.clone(),
        member_id: "staff-orchestrator".into(),
    };
    let custom_error = validate_staff_command(&custom_command, &[custom], &config, &[], "")
        .expect_err("unmapped custom status should have no staff role");
    assert!(custom_error.message.contains("没有可执行员工角色"));
}

#[test]
fn staff_command_policy_reports_unsupported_stage_capability() {
    let task = task_with_status("T-pending", TaskStatus::Pending);
    let command = TaskBoardStaffCommand::StartTask {
        command_id: "start".into(),
        task_id: task.id.clone(),
        member_id: "staff-orchestrator".into(),
    };
    let mut config = TaskBoardStaffConfig::default();
    config.stage_actions = vec![TaskBoardStageAction {
        status: TaskStatus::Pending,
        capability_id: "unsupported".into(),
        ..TaskBoardStageAction::default()
    }];

    let error = validate_staff_command(&command, &[task], &config, &[], "dev_lifecycle")
        .expect_err("unsupported stage capability should be rejected");

    assert!(error.message.contains("不支持的阶段能力 unsupported"));
}
