//! Persistence for the managed MCP catalog.

mod catalog_create;
pub use catalog_create::McpCatalogStore;

#[cfg(test)]
mod catalog_create_tests;
