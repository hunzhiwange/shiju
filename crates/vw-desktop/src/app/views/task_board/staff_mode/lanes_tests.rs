//! 员工模式阶段泳道的测试，覆盖空投影、加载错误和不同分配方式。
//! 用例验证阻塞角色及主责、备份和管理员数量都能稳定展示。
use iced::{Element, Length};

use crate::app::Message;
use crate::app::task::TaskStatus;
use crate::app::task::staff::{
    TaskBoardStaffConfig, TaskBoardStaffLane, TaskBoardStaffProjection, TaskBoardStaffRole,
    TaskBoardStageAssignment,
};

use super::*;

fn lane(
    status: TaskStatus,
    role: Option<TaskBoardStaffRole>,
    display_only: bool,
    task_count: usize,
    blocked_count: usize,
) -> TaskBoardStaffLane {
    TaskBoardStaffLane {
        status,
        role,
        display_only,
        assignments: Vec::new(),
        task_count,
        blocked_count,
    }
}

fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

#[test]
fn lanes_panel_builds_empty_projection_with_load_error() {
    let projection = TaskBoardStaffProjection::default();
    let config = TaskBoardStaffConfig::default();
    let panel = build_lanes_panel(
        &projection,
        &config,
        Some("invalid staff config"),
        &[],
        Length::Fixed(280.0),
        Length::Fill,
    );

    assert_eq!(panel.as_widget().size().width, Length::Fixed(280.0));
    keep_element(panel);
}

#[test]
fn lanes_panel_builds_stage_list_without_load_error() {
    let projection = TaskBoardStaffProjection {
        lanes: vec![
            lane(TaskStatus::Planning, Some(TaskBoardStaffRole::Planner), false, 2, 0),
            lane(TaskStatus::Running, Some(TaskBoardStaffRole::Executor), false, 1, 1),
        ],
        total_task_count: 3,
        blocked_task_count: 1,
        ..TaskBoardStaffProjection::default()
    };
    let config = TaskBoardStaffConfig::default();
    let panel = build_lanes_panel(
        &projection,
        &config,
        None,
        &[TaskStatus::Running],
        Length::Fixed(320.0),
        Length::Fill,
    );

    assert_eq!(panel.as_widget().size().width, Length::Fixed(320.0));
    keep_element(panel);
}

#[test]
fn lane_item_builds_display_only_lane_without_blocked_chip() {
    let lane = lane(TaskStatus::Archived, None, true, 0, 0);
    let config = TaskBoardStaffConfig::default();

    keep_element(build_lane_item(&lane, &config, false));
}

#[test]
fn lane_item_builds_blocked_role_lane_without_stage_assignment() {
    let lane = lane(TaskStatus::Running, Some(TaskBoardStaffRole::Executor), false, 3, 2);
    let config = TaskBoardStaffConfig::default();

    keep_element(build_lane_item(&lane, &config, false));
}

#[test]
fn lane_item_builds_manual_assignment_with_primary_backup_and_manager_counts() {
    let lane = lane(TaskStatus::CodeReview, Some(TaskBoardStaffRole::Reviewer), false, 2, 0);
    let config = TaskBoardStaffConfig {
        stage_assignments: vec![TaskBoardStageAssignment {
            status: TaskStatus::CodeReview,
            primary_member_id: Some("reviewer-primary".to_string()),
            backup_member_ids: vec!["reviewer-backup".to_string()],
            manager_member_ids: vec!["review-manager".to_string()],
            autonomous: false,
        }],
        ..TaskBoardStaffConfig::default()
    };

    keep_element(build_lane_item(&lane, &config, true));
}

#[test]
fn lane_item_builds_autonomous_assignment_without_primary_or_managers() {
    let lane = lane(TaskStatus::Planning, Some(TaskBoardStaffRole::Planner), false, 1, 0);
    let config = TaskBoardStaffConfig {
        stage_assignments: vec![TaskBoardStageAssignment {
            status: TaskStatus::Planning,
            primary_member_id: None,
            backup_member_ids: Vec::new(),
            manager_member_ids: Vec::new(),
            autonomous: true,
        }],
        ..TaskBoardStaffConfig::default()
    };

    keep_element(build_lane_item(&lane, &config, false));
}
