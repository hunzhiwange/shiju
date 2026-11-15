//! Skill metadata, discovery and security auditing.

pub mod agent_metadata;
pub mod audit;
pub mod discovery;

pub use agent_metadata::{
    SkillAgentDependency, SkillAgentInterface, SkillAgentMetadata, SkillAgentPolicy,
};

pub mod cli;
pub mod init;

pub mod types;
pub use types::{PluginSkillSource, Skill, SkillTool};
pub mod local;
pub mod local_sources;
pub mod invoked;
pub mod listing;
pub mod session_state;

pub mod installer;
pub mod loader;
pub mod policy;
pub mod source;

pub mod marketplace;
pub mod prompt;
pub mod session_mounts;
pub mod shiju;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod session_state_tests;
#[cfg(test)]
mod session_mounts_tests;
