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

// 看板与策略测试随实现迁入 task-scheduler。

#[cfg(test)]
#[path = "integration_tests.rs"]
mod integration_tests;

#[cfg(test)]
#[path = "local_e2e_tests.rs"]
mod local_e2e_tests;
