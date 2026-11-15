//! Patch, batch and client tools with typed host boundaries.
#[cfg(not(target_arch = "wasm32"))]
pub mod apply_patch;
pub mod batch;
pub mod client_tool;
pub mod config;
pub mod delegated_tools;
#[cfg(not(target_arch = "wasm32"))]
pub mod question;
pub mod remote_trigger;
pub mod schedule;
pub mod scheduler;
pub mod sleep;
pub mod skill;
pub mod todo;
#[cfg(not(target_arch = "wasm32"))]
pub mod screenshot;

/// Reuses the host's existing Arc adapter without replacing tool instances.
pub trait BatchHost: tool_context::host::ContextHost {
    fn boxed_registry_from_arcs(tools: Vec<std::sync::Arc<dyn tool_types::Tool>>) -> Vec<Box<dyn tool_types::Tool>>;
}
