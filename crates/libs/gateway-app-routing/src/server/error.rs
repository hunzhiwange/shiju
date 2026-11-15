//! HTTP failures shared by application routing endpoints.
use axum::{Json, http::StatusCode, response::{IntoResponse, Response}};
use vw_api_transport::error::{ApiErrorBody, ApiErrorDetail};

pub enum ControlError {
    ClientProjectionUnsupported,
    InvalidInput,
    InvalidConfig,
    UnsupportedApp,
    ConfigNotApplied,
    EgressNotApplied,
    DiagnosticCapacity,
    DiagnosticNotFound,
    Unauthenticated,
    Forbidden,
    NotFound,
    UnsupportedHost,
    AppsAttached,
    AggregationConflict,
    AggregationUnsupported,
    AggregationTargetFailed,
    AddressInUse,
    TransitionInProgress,
    NotReady,
    Unavailable,
}

impl From<StatusCode> for ControlError {
    fn from(status: StatusCode) -> Self {
        match status {
            StatusCode::UNAUTHORIZED => Self::Unauthenticated,
            StatusCode::FORBIDDEN => Self::Forbidden,
            StatusCode::NOT_FOUND => Self::NotFound,
            StatusCode::NOT_IMPLEMENTED => Self::UnsupportedHost,
            _ => Self::Unavailable,
        }
    }
}

impl IntoResponse for ControlError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::ClientProjectionUnsupported => (
                StatusCode::NOT_IMPLEMENTED,
                "client_projection_unsupported",
                "Executor-wide client projection transaction is unavailable",
            ),
            Self::InvalidInput => {
                (StatusCode::BAD_REQUEST, "invalid_input", "Expected an empty JSON object")
            }
            Self::InvalidConfig => {
                (StatusCode::BAD_REQUEST, "invalid_input", "Invalid routing configuration")
            }
            Self::UnsupportedApp => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "unsupported_app",
                "Application routing settings are unsupported",
            ),
            Self::ConfigNotApplied => (
                StatusCode::CONFLICT,
                "config_not_applied",
                "Routing address change could not be applied",
            ),
            Self::EgressNotApplied => (
                StatusCode::CONFLICT,
                "egress_not_applied",
                "Routing egress change could not be applied",
            ),
            Self::DiagnosticCapacity => (
                StatusCode::TOO_MANY_REQUESTS,
                "diagnostic_capacity",
                "The execution host has reached diagnostic capacity",
            ),
            Self::DiagnosticNotFound => {
                (StatusCode::NOT_FOUND, "not_found", "Diagnostic task not found")
            }
            Self::Unauthenticated => {
                (StatusCode::UNAUTHORIZED, "unauthenticated", "Application membership is required")
            }
            Self::Forbidden => (StatusCode::FORBIDDEN, "forbidden", "Routing permission denied"),
            Self::NotFound => (StatusCode::NOT_FOUND, "not_found", "Execution host not found"),
            Self::UnsupportedHost => {
                (StatusCode::NOT_IMPLEMENTED, "unsupported_host", "Execution host is unsupported")
            }
            Self::AppsAttached => {
                (StatusCode::CONFLICT, "apps_attached", "Applications are still attached")
            }
            Self::AggregationConflict => (
                StatusCode::CONFLICT,
                "revision_conflict",
                "Application aggregation changed or requires recovery",
            ),
            Self::AggregationUnsupported => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "unsupported_target",
                "Application aggregation recovery is unsupported",
            ),
            Self::AggregationTargetFailed => (
                StatusCode::BAD_GATEWAY,
                "target_failed",
                "Application aggregation restoration failed",
            ),
            Self::AddressInUse => {
                (StatusCode::CONFLICT, "address_in_use", "Routing address is in use")
            }
            Self::TransitionInProgress => (
                StatusCode::CONFLICT,
                "transition_in_progress",
                "Routing transition is in progress",
            ),
            Self::NotReady => {
                (StatusCode::SERVICE_UNAVAILABLE, "not_ready", "Routing listener is not running")
            }
            Self::Unavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "unavailable",
                "Application routing control is unavailable",
            ),
        };
        (
            status,
            Json(ApiErrorBody {
                error: ApiErrorDetail { code: code.into(), message: message.into(), details: None },
            }),
        )
            .into_response()
    }
}
