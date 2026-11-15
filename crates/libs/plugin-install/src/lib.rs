//! Plugin installation compatibility and lifecycle primitives.

pub mod identity;
pub mod host_files;
pub mod marketplace;
#[cfg(test)]
mod marketplace_tests;
pub mod installed_state;
pub mod agents;
pub mod discovery;
pub mod git_source;

pub mod commands;
pub mod dependencies;

pub mod skill_frontmatter;
pub mod skill_invocation;
pub mod skill_roots;
