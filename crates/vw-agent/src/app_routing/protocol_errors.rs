//! Client-facing errors for the application data plane.

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::{Value, json};

#[derive(Clone, Copy)]
pub(super) enum Protocol {
    Messages,
    Responses,
    Chat,
    Gemini,
}

impl Protocol {
    pub(super) fn for_path(path: &str) -> Option<Self> {
        match path {
            "/v1/messages"
            | "/claude/v1/messages"
            | "/claude-desktop/v1/messages"
            | "/claude-desktop/v1/models" => Some(Self::Messages),
            "/responses"
            | "/v1/responses"
            | "/v1/v1/responses"
            | "/codex/v1/responses"
            | "/grokbuild/v1/responses"
            | "/responses/compact"
            | "/v1/responses/compact"
            | "/v1/v1/responses/compact"
            | "/codex/v1/responses/compact"
            | "/grokbuild/v1/responses/compact" => Some(Self::Responses),
            "/chat/completions"
            | "/v1/chat/completions"
            | "/v1/v1/chat/completions"
            | "/codex/v1/chat/completions" => Some(Self::Chat),
            "/v1beta/models" | "/gemini/v1beta/models" | "/gemini/v1/models" => Some(Self::Gemini),
            path if path.starts_with("/v1beta/models/")
                || path.starts_with("/gemini/v1beta/models/")
                || path.starts_with("/gemini/v1/models/") =>
            {
                Some(Self::Gemini)
            }
            _ => None,
        }
    }

    fn json(self, error: RouteError) -> Value {
        let (code, message) = error.details();
        let status = error.status();
        match self {
            Self::Messages => json!({
                "type": "error",
                "error": {"type": messages_type(status), "message": message},
            }),
            Self::Responses | Self::Chat => json!({
                "error": {"type": openai_type(status), "code": code, "message": message, "param": null},
            }),
            Self::Gemini => json!({
                "error": {"code": status.as_u16(), "status": gemini_status(status), "message": message},
            }),
        }
    }

    pub(super) fn terminal(self, error: RouteError, sequence_number: u64) -> String {
        match self {
            Self::Messages => format!("event: error\ndata: {}\n\n", self.json(error)),
            Self::Responses => {
                let (code, message) = error.details();
                let event = json!({
                    "type": "response.failed",
                    "sequence_number": sequence_number,
                    "response": {
                        "object": "response",
                        "status": "failed",
                        "output": [],
                        "error": {"code": code, "message": message},
                    },
                });
                format!("event: response.failed\ndata: {event}\n\n")
            }
            Self::Chat | Self::Gemini => format!("data: {}\n\n", self.json(error)),
        }
    }
}

/// Only status and origin cross the error boundary. Upstream headers, bodies,
/// URLs, and transport error strings never enter this type.
#[derive(Clone, Copy)]
pub(super) enum RouteError {
    Status(StatusCode),
    UpstreamStatus(StatusCode),
    StreamFailed,
    UnreadableAgentTask,
    ClassifierLoop,
}

impl From<StatusCode> for RouteError {
    fn from(status: StatusCode) -> Self {
        Self::Status(status)
    }
}

impl RouteError {
    pub(super) fn check_upstream(status: StatusCode) -> Result<(), Self> {
        if status.is_success() { Ok(()) } else { Err(Self::UpstreamStatus(status)) }
    }

    pub(super) fn status(self) -> StatusCode {
        match self {
            Self::Status(status) => status,
            Self::UnreadableAgentTask | Self::ClassifierLoop => StatusCode::BAD_REQUEST,
            Self::UpstreamStatus(
                status @ (StatusCode::PAYLOAD_TOO_LARGE | StatusCode::UNAUTHORIZED),
            ) => status,
            Self::UpstreamStatus(StatusCode::GATEWAY_TIMEOUT | StatusCode::REQUEST_TIMEOUT) => {
                StatusCode::GATEWAY_TIMEOUT
            }
            Self::UpstreamStatus(_) | Self::StreamFailed => StatusCode::BAD_GATEWAY,
        }
    }

    fn details(self) -> (&'static str, &'static str) {
        match self {
            Self::ClassifierLoop => ("classifier_retry_loop", vw_agent_apps::routing::classifier_guard::CLASSIFIER_LOOP_MESSAGE),
            Self::UnreadableAgentTask => ("unreadable_encrypted_agent_task", "The current provider cannot read the encrypted agent task. Use the main agent provider for this task."),
            Self::UpstreamStatus(StatusCode::PAYLOAD_TOO_LARGE) => (
                "upstream_request_too_large",
                "The request exceeds the upstream provider's size limit.",
            ),
            Self::Status(StatusCode::PAYLOAD_TOO_LARGE) => {
                ("local_request_too_large", "The request exceeds the local routing size limit.")
            }
            Self::UpstreamStatus(StatusCode::UNAUTHORIZED) => (
                "upstream_authentication_failed",
                "The upstream provider rejected its configured credential.",
            ),
            Self::Status(StatusCode::UNAUTHORIZED) => {
                ("invalid_api_key", "A valid application routing credential is required.")
            }
            Self::Status(StatusCode::GATEWAY_TIMEOUT)
            | Self::UpstreamStatus(StatusCode::GATEWAY_TIMEOUT | StatusCode::REQUEST_TIMEOUT) => {
                ("upstream_timeout", "The upstream response deadline elapsed.")
            }
            Self::Status(StatusCode::REQUEST_TIMEOUT) => {
                ("request_timeout", "The request timed out.")
            }
            Self::StreamFailed => {
                ("upstream_stream_failed", "The upstream stream failed before completion.")
            }
            Self::UpstreamStatus(_) | Self::Status(StatusCode::BAD_GATEWAY) => {
                ("upstream_error", "The upstream provider could not serve the request.")
            }
            Self::Status(status) => (
                "routing_error",
                status.canonical_reason().unwrap_or("Application routing request failed."),
            ),
        }
    }

    pub(super) fn response(self, protocol: Option<Protocol>) -> Response {
        let status = self.status();
        match protocol {
            Some(protocol) => (status, Json(protocol.json(self))).into_response(),
            None => status.into_response(),
        }
    }
}

fn messages_type(status: StatusCode) -> &'static str {
    match status {
        StatusCode::UNAUTHORIZED => "authentication_error",
        StatusCode::FORBIDDEN => "permission_error",
        StatusCode::NOT_FOUND => "not_found_error",
        StatusCode::PAYLOAD_TOO_LARGE => "request_too_large",
        StatusCode::TOO_MANY_REQUESTS => "rate_limit_error",
        StatusCode::SERVICE_UNAVAILABLE => "overloaded_error",
        status if status.is_client_error() => "invalid_request_error",
        _ => "api_error",
    }
}

fn openai_type(status: StatusCode) -> &'static str {
    match status {
        StatusCode::TOO_MANY_REQUESTS => "rate_limit_error",
        status if status.is_client_error() => "invalid_request_error",
        _ => "server_error",
    }
}

fn gemini_status(status: StatusCode) -> &'static str {
    match status {
        StatusCode::UNAUTHORIZED => "UNAUTHENTICATED",
        StatusCode::FORBIDDEN => "PERMISSION_DENIED",
        StatusCode::NOT_FOUND => "NOT_FOUND",
        StatusCode::PAYLOAD_TOO_LARGE | StatusCode::TOO_MANY_REQUESTS => "RESOURCE_EXHAUSTED",
        StatusCode::REQUEST_TIMEOUT | StatusCode::GATEWAY_TIMEOUT => "DEADLINE_EXCEEDED",
        StatusCode::SERVICE_UNAVAILABLE | StatusCode::BAD_GATEWAY => "UNAVAILABLE",
        status if status.is_client_error() => "INVALID_ARGUMENT",
        _ => "INTERNAL",
    }
}
