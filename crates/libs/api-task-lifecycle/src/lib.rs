//! 任务生命周期 workflow API 契约。

pub mod task_lifecycle;

pub use task_lifecycle::{
    TaskLifecycleStatusRoleDto,
    TaskLifecycleStatusTransitionDto,
    TaskLifecycleStatusDto,
    TaskLifecycleWorkflowDto,
    TaskLifecycleWorkflowUpsertBody,
    TaskLifecycleWorkflowDeleteResponse,
};

#[cfg(test)]
mod task_lifecycle_tests;
