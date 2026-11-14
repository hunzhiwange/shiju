//! Project configuration composition; storage and authorized host dispatch stay
//! with the agent. This domain is separate from runtime AgentProfile identities.
pub mod apply_provider;
pub mod rename;
pub mod snapshot;
pub mod create;
pub mod preview;
pub mod resnapshot;
pub mod autosave;
pub mod apply_skills;

pub mod apply_mcp;
