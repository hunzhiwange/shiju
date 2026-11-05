//! Fixed public errors for the icon API. Never retain input or error sources.
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use vw_api_types::agent_apps::{AgentAppsErrorCode, AgentAppsErrorResponse};

/// Icon-domain HTTP boundary, accepting only stable business codes.
#[derive(Debug)]
pub struct AgentAppsApiError(AgentAppsErrorCode);

impl From<AgentAppsErrorCode> for AgentAppsApiError {
    fn from(code: AgentAppsErrorCode) -> Self {
        Self(code)
    }
}

impl AgentAppsApiError {
    /// Discard storage diagnostics before logging or constructing the response.
    pub fn storage_unavailable(_source: anyhow::Error) -> Self {
        Self(AgentAppsErrorCode::StorageUnavailable)
    }
}

impl IntoResponse for AgentAppsApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self.0 {
            AgentAppsErrorCode::InvalidInput => (StatusCode::BAD_REQUEST, "Invalid icon input"),
            AgentAppsErrorCode::NotFound => (StatusCode::NOT_FOUND, "Icon not found"),
            AgentAppsErrorCode::Conflict => (StatusCode::CONFLICT, "Icon library conflict"),
            AgentAppsErrorCode::StorageUnavailable => {
                (StatusCode::SERVICE_UNAVAILABLE, "Icon storage unavailable")
            }
        };
        tracing::warn!(error_category = ?self.0, "icon API request failed");
        (status, Json(AgentAppsErrorResponse { code: self.0, message: message.into() }))
            .into_response()
    }
}
