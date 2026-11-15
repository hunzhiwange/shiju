//! Plugin hook protocol, matching, asynchronous ownership and content trust.

pub mod hook_async;
pub mod hook_matcher;
pub mod hook_permission;
pub mod hook_post_tool;
pub mod hook_pre_tool;
pub mod hook_trust_content;
pub mod hook_command;
pub mod hook_compact;
pub mod hook_mcp;
pub mod hook_model;
pub mod hook_prompt;
pub mod hook_session;
pub mod hook_stop;
pub mod hook_subagent;
pub mod hook_manifest;

pub mod host;
