//! 任务生命周期 workflow 领域服务。
//!
//! 本模块只承载 `vw-agent` 内部生命周期预设、注册表合并和纯校验逻辑，
//! 不连接 HTTP、SQLite 或桌面端任务状态枚举。

pub mod builtin;
pub mod registry;
pub mod store;

pub use builtin::{
    DEFAULT_TASK_LIFECYCLE_WORKFLOW_ID, built_in_workflow_by_id, built_in_workflows,
    is_built_in_workflow_id,
};
pub use registry::TaskLifecycleRegistry;

#[cfg(test)]
#[path = "registry_tests.rs"]
mod registry_tests;
