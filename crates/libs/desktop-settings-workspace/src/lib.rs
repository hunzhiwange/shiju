//! Workspace settings state, reducers and views.
pub mod state;
pub mod messages;
pub mod worktrees;
pub mod environment;
pub mod system_settings_worktrees;
#[cfg(test)] mod worktrees_tests;
pub mod system_settings_projects;
pub mod projects;
pub mod managed;
pub mod recovery;
pub mod lifecycle;
#[cfg(test)] mod recovery_tests;
