//! Agent Scheduler HTTP 路由与服务注入契约。

#[cfg(not(target_arch = "wasm32"))]
pub mod store_gateway;

mod types;
pub use types::*;

#[cfg(not(target_arch = "wasm32"))]
mod gateway;
#[cfg(not(target_arch = "wasm32"))]
pub use gateway::AgentSchedulerGateway;
#[cfg(not(target_arch = "wasm32"))]
mod routes;
#[cfg(not(target_arch = "wasm32"))]
pub use routes::{router_with_gateway, agent_scheduler_project_task_graph,
    agent_scheduler_project_replace_task_graph, agent_scheduler_project_create_task,
    agent_scheduler_project_drain_outbox};

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "routes_tests.rs"]
mod routes_tests;
