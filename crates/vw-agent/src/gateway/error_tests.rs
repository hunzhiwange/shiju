// 验证 API 错误构造器、状态码映射与响应正文契约。

// 响应检查同时约束 HTTP 状态和 JSON 错误正文，避免只修正其中一层。
use super::*;
use axum::body::to_bytes;
use axum::response::IntoResponse;
use serde_json::json;

#[test]
fn bad_request_sets_status_and_message() {
    let error = ApiError::bad_request("bad input");

    assert_eq!(error.status, axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(error.to_string(), "bad input");
}

#[test]
fn not_found_sets_status_and_message() {
    let error = ApiError::not_found("missing");

    assert_eq!(error.status, axum::http::StatusCode::NOT_FOUND);
    assert_eq!(error.to_string(), "missing");
}

#[test]
fn internal_sets_status_and_message() {
    let error = ApiError::internal("storage unavailable");

    assert_eq!(error.status, axum::http::StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(error.to_string(), "storage unavailable");
}

#[test]
fn not_implemented_sets_status_and_message() {
    let error = ApiError::not_implemented("planned capability");

    assert_eq!(error.status, axum::http::StatusCode::NOT_IMPLEMENTED);
    assert_eq!(error.to_string(), "planned capability");
}

#[test]
fn constructor_accepts_owned_message() {
    let error = ApiError::bad_request(String::from("owned input"));

    assert_eq!(error.status, axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(error.to_string(), "owned input");
}

#[test]
fn api_error_implements_std_error() {
    fn assert_error(error: &dyn std::error::Error) -> String {
        error.to_string()
    }

    let error = ApiError::bad_request("invalid");

    assert_eq!(assert_error(&error), "invalid");
}

#[tokio::test]
async fn into_response_uses_status_and_error_json_body() {
    let response = ApiError::not_found("missing").into_response();

    assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(body, json!({ "error": "missing" }));
}

#[tokio::test]
async fn into_response_preserves_special_characters_in_json_error_body() {
    let response = ApiError::bad_request("bad \"input\"\ntry again").into_response();

    assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(body, json!({ "error": "bad \"input\"\ntry again" }));
}
