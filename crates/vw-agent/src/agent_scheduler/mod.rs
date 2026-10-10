//! 自主代理调度子系统，统一组织领域模型、存储、派发、运行、恢复与策略执行。
//! 外部系统通过适配器和服务入口接入，领域层不直接依赖具体控制面或数据库。
//!
//! Agent Scheduler internal scheduling model.
//!
//! This module is the source of truth for tasks, runs, resource leases, events,
//! and local execution facts.

pub mod adapters;
pub mod artifacts;
pub mod domain;
pub mod engine;
pub mod events;
pub mod lifecycle_bridge;
pub mod outbox_dispatcher;
pub mod policy;
pub mod resources;
pub mod runner;
pub mod scheduler;
pub mod state_machine;
pub mod store;
pub mod task_board;
pub mod task_graph;

#[cfg(test)]
#[path = "task_board_tests.rs"]
mod task_board_tests;

#[cfg(test)]
#[path = "domain_tests.rs"]
mod domain_tests;

#[cfg(test)]
#[path = "state_machine_tests.rs"]
mod state_machine_tests;

#[cfg(test)]
#[path = "store/schema_tests.rs"]
mod schema_tests;

#[cfg(test)]
#[path = "store/tasks_tests.rs"]
mod tasks_tests;

#[cfg(test)]
#[path = "store/runs_tests.rs"]
mod runs_tests;

#[cfg(test)]
#[path = "store/resources_tests.rs"]
mod resources_tests;

#[cfg(test)]
#[path = "store/events_tests.rs"]
mod events_tests;

#[cfg(test)]
#[path = "store/outbox_tests.rs"]
mod store_outbox_tests;

#[cfg(test)]
#[path = "store/logs_tests.rs"]
mod store_logs_tests;

#[cfg(test)]
#[path = "store/artifacts_tests.rs"]
mod store_artifacts_tests;

#[cfg(test)]
#[path = "policy_tests.rs"]
mod policy_tests;

#[cfg(test)]
#[path = "lifecycle_bridge_tests.rs"]
mod lifecycle_bridge_tests;

#[cfg(test)]
#[path = "artifacts_tests.rs"]
mod artifacts_tests;

#[cfg(test)]
#[path = "integration_tests.rs"]
mod integration_tests;

#[cfg(test)]
#[path = "local_e2e_tests.rs"]
mod local_e2e_tests;

#[cfg(test)]
#[path = "runner/local_tests.rs"]
mod local_runner_tests;

#[cfg(test)]
#[path = "task_graph_tests.rs"]
mod task_graph_tests;
