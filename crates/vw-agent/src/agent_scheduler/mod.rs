//! Agent Scheduler internal scheduling model.
//!
//! This module is the source of truth for Agent Teams style scheduling facts:
//! tasks, runs, workers, teams, resource leases, events, and Matrix outbox
//! records. It is the only scheduling and local execution fact source.

pub mod adapters;
pub mod artifacts;
pub mod capabilities;
pub mod domain;
pub mod engine;
pub mod events;
pub mod lifecycle_bridge;
pub mod outbox_dispatcher;
pub mod policy;
pub mod resources;
pub mod runner;
pub mod scheduler;
pub mod staff;
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
#[path = "store/workers_tests.rs"]
mod workers_tests;

#[cfg(test)]
#[path = "store/teams_tests.rs"]
mod teams_tests;

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
#[path = "outbox_dispatcher_tests.rs"]
mod outbox_dispatcher_tests;

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
#[path = "scheduler/worker_selection_tests.rs"]
mod worker_selection_tests;

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
#[path = "runner/matrix_tests.rs"]
mod matrix_runner_tests;

#[cfg(test)]
#[path = "task_graph_tests.rs"]
mod task_graph_tests;
