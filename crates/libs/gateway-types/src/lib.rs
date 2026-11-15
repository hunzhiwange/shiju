//! Shared gateway errors, request types, limits and strongly typed state handles.
#![cfg(not(target_arch = "wasm32"))]
pub mod error;
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
