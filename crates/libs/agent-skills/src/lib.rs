//! Skill metadata, discovery and security auditing.

pub mod agent_metadata;
pub mod audit;
pub mod discovery;

pub use agent_metadata::{
    SkillAgentDependency, SkillAgentInterface, SkillAgentMetadata, SkillAgentPolicy,
};

pub mod cli;
pub mod init;
