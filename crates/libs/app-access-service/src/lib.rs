//! Shared application authorization service.
//!
//! Owns administrator policy, permits and host resolution; contains no HTTP
//! handlers, router/bootstrap or gateway state. Route packages consume this
//! service directly without depending on another route implementation.
#![cfg(not(target_arch = "wasm32"))]
pub mod access;

/// Maximum number of MCP probes authorized and executed in one job.
pub const MAX_PROBES_PER_JOB: usize = 32;

pub use agent_app_services::usage_settings::UsageSigningCredentialRefs;

pub mod account_host;
