//! 任务看板员工模式模型。

pub mod commands;
pub mod events;
pub mod models;
pub mod routing;
pub mod scheduling;
pub mod stage_capabilities;

pub use commands::{
    TaskBoardStaffCommand, TaskBoardStaffCommandAction, TaskBoardStaffCommandError,
    validate_staff_command,
};
pub use events::{
    TaskBoardStaffCommandGroup, TaskBoardStaffCommandQueue, TaskBoardStaffCommandStatus,
    TaskBoardStaffEvent, TaskBoardStaffEventKind, TaskBoardStaffEventStatus,
    TaskBoardStaffMessageDeliveryStatus, TaskBoardStaffMessageEvent,
    TaskBoardStaffOrchestrationContext,
};
pub use models::{
    TaskBoardStaffCapability, TaskBoardStaffCapabilityDraftField, TaskBoardStaffCapabilityKind,
    TaskBoardStaffCapabilitySummary, TaskBoardStaffCapabilitySummaryKind, TaskBoardStaffConfig,
    TaskBoardStaffLevel, TaskBoardStaffMember, TaskBoardStaffRole, TaskBoardStaffStatus,
    TaskBoardStageAssignment, TaskBoardStageWorkflowBinding, TaskBoardWorkflowStage,
    apply_task_board_staff_config_to_staff_config,
    apply_task_board_staff_config_to_staff_config_for_workflow_id,
    task_board_staff_capabilities_from_gateway_response,
    task_board_staff_capability_catalog_request, task_board_staff_config_from_staff_config,
    task_board_staff_config_from_staff_config_for_workflow_id,
};
#[cfg(test)]
use models::{TaskBoardStageAction, workflow_stages_for_workflow_id};
pub use routing::{
    TaskBoardStaffActionAvailability, TaskBoardStaffAssignment, TaskBoardStaffLane,
    TaskBoardStaffMemberLoad, TaskBoardStaffProjection, build_staff_mode_projection,
};
#[cfg(test)]
use scheduling::push_command;
pub use scheduling::{
    TaskBoardStaffAutoSchedulePlan, TaskBoardStaffAutoScheduleSkip, build_staff_auto_schedule_plan,
};
pub use stage_capabilities::{
    StageCapability, StageCapabilityId, StageCapabilityRegistry, stage_capability_registry,
};
#[cfg(test)]
#[path = "models_tests.rs"]
mod models_tests;

#[cfg(test)]
#[path = "routing_tests.rs"]
mod routing_tests;

#[cfg(test)]
#[path = "events_tests.rs"]
mod events_tests;

#[cfg(test)]
#[path = "commands_tests.rs"]
mod commands_tests;

#[cfg(test)]
#[path = "scheduling_tests.rs"]
mod scheduling_tests;

#[cfg(test)]
#[path = "stage_capabilities_tests.rs"]
mod stage_capabilities_tests;
