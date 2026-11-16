//! 自主代理调度领域契约与审计事件。

pub mod domain;

#[cfg(not(target_arch = "wasm32"))]
pub mod task_graph;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod task_graph_tests;
pub mod events;

#[cfg(test)]
mod domain_tests;

pub mod state_machine;
pub mod task_records;
pub mod resource_kind;
pub mod artifacts;
pub mod resources;

#[cfg(test)]
mod state_machine_tests;
#[cfg(test)]
mod artifacts_tests;

pub mod policy;
pub mod policy_port;
