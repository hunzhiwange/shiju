//! Managed MCP catalog, distribution, runtime selection and ACP usage contracts.

pub mod mcp_management;
pub use mcp_management::*;

#[cfg(test)]
mod mcp_management_tests;
