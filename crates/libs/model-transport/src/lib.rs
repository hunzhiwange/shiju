//! Authorized model HTTP transport and account-scoped wire policies.
pub mod binding;
pub mod upstream;
pub mod quota_gate;
pub mod codex_oauth_transport;
pub mod xai_oauth_transport;
#[cfg(target_os = "macos")]
pub mod copilot_transport;
pub use binding::{RelayBinding, RelayIdentity};
pub use upstream::upstream_http_client;
#[cfg(test)]
mod binding_tests;
#[cfg(all(test, target_os = "macos"))]
mod copilot_transport_tests;

pub mod stream_observer;

pub mod gemini_oauth_transport;

pub mod credential;

pub mod routing_selection;

pub mod execution;
pub mod provider_transport;
