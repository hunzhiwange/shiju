//! Project configuration composition; storage and authorized host dispatch stay
//! with the agent. This domain is separate from runtime AgentProfile identities.
pub mod apply_provider;
pub mod rename;
pub mod rename;
pub mod snapshot;
pub mod resnapshot;
pub mod autosave;
pub mod apply_skills;

pub mod apply_mcp;

pub mod apply_prompt;

pub mod apply;
