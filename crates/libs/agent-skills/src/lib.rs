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
pub mod invoked;
pub mod listing;
pub mod session_state;

pub mod installer;
pub mod loader;
pub mod policy;
pub mod source;
