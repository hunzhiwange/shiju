//! Gateway timeout selection and skey authentication policy.
use gateway_types::AppState;
use axum::{
    body::Body,
    extract::{Request, State},
    http::Method,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::time::Duration;

/// 第三方 webhook 请求允许的最大正文大小（32 MiB）。
pub use gateway_types::MAX_BODY_SIZE;
/// 普通请求的默认超时秒数，用于阻止慢速请求长期占用连接。
pub const REQUEST_TIMEOUT_SECS: u64 = 30;
/// 聊天、工作流和任务计划 SSE 请求允许的最长执行秒数。
pub const WORKFLOW_CHAT_MESSAGES_TIMEOUT_SECS: u64 = 3_600;

pub async fn request_timeout_middleware(req: Request<Body>, next: Next) -> Response {
    let timeout = request_timeout_for_path(req.uri().path());
    match tokio::time::timeout(timeout, next.run(req)).await {
        Ok(response) => response,
        Err(_) => StatusCode::REQUEST_TIMEOUT.into_response(),
    }
}

pub fn request_timeout_for_path(path: &str) -> Duration {
    if is_long_running_sse_path(path) {
        Duration::from_secs(WORKFLOW_CHAT_MESSAGES_TIMEOUT_SECS)
    } else if path
        .strip_prefix("/v1/desktop/acp-install/")
        .is_some_and(|agent| !agent.is_empty() && !agent.contains('/'))
    {
        // Native installers already enforce a five-minute deadline and kill their process tree.
        Duration::from_secs(310)
    } else {
        Duration::from_secs(REQUEST_TIMEOUT_SECS)
    }
}

fn is_long_running_sse_path(path: &str) -> bool {
    is_workflow_chat_messages_path(path)
        || path == "/v1/chat/stream"
        || path == "/v1/task-plan-run/stream"
}

pub fn is_workflow_chat_messages_path(path: &str) -> bool {
    path == "/v1/workflow/applications/chat-messages"
        || (path.starts_with("/v1/workflow/applications/")
            && path.ends_with("/chat-messages")
            && path
                .trim_end_matches("/chat-messages")
                .trim_start_matches("/v1/workflow/applications/")
                .contains(|ch: char| !ch.is_whitespace()))
}

pub async fn skey_auth_middleware<S: Send + Sync, C: Send + Sync, H: Send + Sync>(
    State(state): State<AppState<S, C, H>>,
    req: Request<Body>,
    next: Next,
) -> Response {
    if req.method() == Method::OPTIONS || is_public_skey_auth_path(req.uri().path()) {
        return next.run(req).await;
    }

    if let Err(err) = crate::api::auth::require_auth(&state, req.headers()) {
        return err.into_response();
    }

    next.run(req).await
}

pub fn is_public_skey_auth_path(path: &str) -> bool {
    matches!(path, "/v1/global/health" | "/global/health")
}
