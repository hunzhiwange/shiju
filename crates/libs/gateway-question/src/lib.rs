//! 问题管理路由模块
#![cfg(not(target_arch = "wasm32"))]

use axum::Json;
use axum::Router;
use axum::extract::Path;
use axum::routing::{get, post};
use serde::Deserialize;

use agent_question as question;
use gateway_types::ApiError;

/// 从宿主可信执行上下文读取当前租户，沿用原问题路由的租户语义。
pub trait QuestionHost: Send + Sync + 'static {
    fn current_tenant_scope() -> vw_api_transport::TenantScope;
}

pub fn router<S, H: QuestionHost>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/question", get(question_list::<H>))
        .route("/question/{request_id}/reply", post(question_reply::<H>))
        .route("/question/{request_id}/reject", post(question_reject::<H>))
}

async fn question_list<H: QuestionHost>() -> Json<Vec<question::Request>> {
    Json(question::list(H::current_tenant_scope()))
}

#[derive(Debug, Deserialize)]
struct QuestionReplyRequest {
    answers: Vec<Vec<String>>,
}

async fn question_reply<H: QuestionHost>(
    Path(request_id): Path<String>,
    Json(body): Json<QuestionReplyRequest>,
) -> Result<Json<bool>, ApiError> {
    question::reply(
        question::ReplyInput { request_id, answers: body.answers },
        H::current_tenant_scope(),
    );
    Ok(Json(true))
}

async fn question_reject<H: QuestionHost>(
    Path(request_id): Path<String>,
) -> Result<Json<bool>, ApiError> {
    question::reject(request_id, H::current_tenant_scope());
    Ok(Json(true))
}

#[cfg(test)]
#[path = "question_tests.rs"]
mod question_tests;
