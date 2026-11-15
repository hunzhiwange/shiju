//! 自主代理任务调度、持久化、派发与恢复。
#![cfg(not(target_arch = "wasm32"))]

pub use task_scheduler_core::{domain, events, state_machine};
pub mod adapters;
pub mod engine;
pub mod lifecycle_bridge;
pub mod outbox_dispatcher;
pub mod store;
pub mod task_graph;
#[cfg(test)]
mod lifecycle_bridge_tests;
#[cfg(test)]
mod task_graph_tests;

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
mod outbox_tests;

#[cfg(test)]
#[path = "store/logs_tests.rs"]
mod logs_tests;

#[cfg(test)]
#[path = "store/artifacts_tests.rs"]
mod artifacts_tests;

pub mod runner;
pub mod scheduler;
#[cfg(test)]
#[path = "runner/local_tests.rs"]
mod local_runner_tests;
