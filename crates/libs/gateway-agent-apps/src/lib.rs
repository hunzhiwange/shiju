//! Application gateway composition exports. Shared authorization owns no handlers.
#![cfg(not(target_arch = "wasm32"))]
pub use app_access_service::{access, UsageSigningCredentialRefs, MAX_PROBES_PER_JOB};
