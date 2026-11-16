//! Authorized executable catalogs and tool discovery.
pub mod catalog;
pub mod discovery;
pub mod tool_search;
pub use tool_types::{Tool, ToolCallResult, ToolRenderHint, ToolResult, ToolSpec};
pub use tool_context::{ExecutedToolCall, ToolCallError};
pub mod registry;
pub mod ids;
pub use ids::*;
pub mod toolset;
pub mod security;
pub mod mcp;
pub mod agents;
pub mod factory;
pub mod runtime;
pub mod hooks;

#[cfg(test)]
mod host_migration_33_tests;

#[cfg(test)]
mod host_migration_34_tests;

#[cfg(test)]
mod host_migration_35_tests;

#[cfg(test)]
mod host_migration_32_tests;
