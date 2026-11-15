//! Shared application authorization service.
//!
//! Owns administrator policy, permits and host resolution; contains no HTTP
//! handlers, router/bootstrap or gateway state. Route packages consume this
//! service directly without depending on another route implementation.
#![cfg(not(target_arch = "wasm32"))]
pub mod access;

/// Maximum number of MCP probes authorized and executed in one job.
pub const MAX_PROBES_PER_JOB: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UsageSigningCredentialRefs {
    pub access_key_id_ref: vw_api_apps_usage::apps_usage::settings::UsageCredentialId,
    pub secret_access_key_ref: vw_api_apps_usage::apps_usage::settings::UsageCredentialId,
}
