//! Shared gateway errors, request types, limits and strongly typed state handles.
#![cfg(not(target_arch = "wasm32"))]
pub mod error;

/// Shared Bearer skey extraction for gateway request boundaries.
pub mod auth {
    use axum::http::{HeaderMap, header};

    /// 从 `Authorization` 头提取非空 Bearer skey。
    pub fn extract_auth_skey(headers: &HeaderMap) -> Option<&str> {
        headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .map(str::trim)
            .and_then(|auth| auth.strip_prefix("Bearer "))
            .map(str::trim)
            .filter(|skey| !skey.is_empty())
    }
}

pub mod limits;
pub mod options;
pub mod state;
pub mod types;

pub use error::ApiError;
pub use limits::*;
pub use options::ServeOptions;
pub use state::{AppState, GatewayChannels};
pub use types::{AgentBody, WebhookBody};

pub mod app_operations;
pub mod tenant_scope;
