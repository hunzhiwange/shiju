use axum::{Router, routing::{get,post,put}};
use super::message_ops::Host;

#[tokio::test]
async fn acp_model_source_ui_http_persists_and_rejects_without_local_fallback() {
    let router = Router::new()
        .route("/session", post(super::session_ops::ui_session_create))
        .route("/session/{session_id}/acp-model-source", put(super::acp_options::set_source))
        .route("/session/{session_id}/acp-options", get(super::acp_options::get));
    gateway_session::acp_source_acceptance::acp_model_source_ui_http_persists_and_rejects_without_local_fallback::<Host>(router).await;
}

#[tokio::test]
async fn acp_public_permission_options_reject_direct_put_and_chat_over_http() {
    let router = Router::new()
        .route("/session", post(super::session_ops::ui_session_create))
        .route("/session/{session_id}/acp-options", put(super::acp_options::set))
        .route("/chat/stream", post(super::stream::chat_stream));
    gateway_session::acp_source_acceptance::acp_public_permission_options_reject_direct_put_and_chat_over_http::<Host>(router).await;
}
