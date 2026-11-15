//! 任务生命周期 workflow 领域服务。
//!
//! 承载生命周期预设、注册表校验与租户隔离的 SQLite/PostgreSQL 存储。
//! HTTP 错误映射由调用方完成。

mod error;
pub use error::TaskLifecycleError;

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
