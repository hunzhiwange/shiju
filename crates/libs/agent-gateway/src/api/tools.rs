//! Shared authentication and diagnostics binding used by the host routes.
use agent_config::runtime::Config;
use agent_doctor as doctor;
use axum::{Json, extract::State, http::{HeaderMap, StatusCode}, response::IntoResponse};
use gateway_types::AppState;
struct ToolsHost;

impl<S, C, H> gateway_tools::ToolsHost<S, C, H> for ToolsHost {
    type Diagnostic = doctor::DiagResult;

    fn require_auth(
        state: &gateway_types::AppState<S, C, H>,
        headers: &HeaderMap,
    ) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
        super::auth::require_auth(state, headers)
    }

    fn diagnose(config: &Config) -> Vec<Self::Diagnostic> {
        doctor::diagnose(config)
    }

    fn severity(result: &Self::Diagnostic) -> gateway_tools::Severity {
        match result.severity {
            doctor::Severity::Ok => gateway_tools::Severity::Ok,
            doctor::Severity::Warn => gateway_tools::Severity::Warn,
            doctor::Severity::Error => gateway_tools::Severity::Error,
        }
    }
}

pub async fn handle_api_tools<S, C, H>(state: State<AppState<S, C, H>>, headers: HeaderMap) -> impl IntoResponse { gateway_tools::handle_api_tools::<_, _, _, ToolsHost>(state, headers).await }
pub async fn handle_api_doctor<S, C, H>(state: State<AppState<S, C, H>>, headers: HeaderMap) -> impl IntoResponse { gateway_tools::handle_api_doctor::<_, _, _, ToolsHost>(state, headers).await }
