// 覆盖提问请求的创建、回答、拒绝及缺失项处理。

use super::*;
use axum::body::Body;
use axum::extract::Path;
use axum::http::{Method, Request, StatusCode};
use tokio::time::{Duration, sleep};
use tower::ServiceExt;

#[test]
fn router_builds_with_app_state() {
    let _: axum::Router<()> = router();
}

#[test]
fn question_reply_request_deserializes_answers() {
    let body: QuestionReplyRequest =
        serde_json::from_value(serde_json::json!({"answers": [["yes"]]})).expect("valid reply");

    assert_eq!(body.answers, vec![vec!["yes".to_string()]]);
}

#[test]
fn question_reply_request_preserves_grouped_and_empty_answers() {
    let body: QuestionReplyRequest = serde_json::from_value(serde_json::json!({
        "answers": [["yes", "later"], [], ["自定义答案"]]
    }))
    .expect("grouped reply");

    assert_eq!(
        body.answers,
        vec![
            vec!["yes".to_string(), "later".to_string()],
            Vec::<String>::new(),
            vec!["自定义答案".to_string()],
        ]
    );
}

#[test]
fn question_reply_request_rejects_missing_or_flat_answers() {
    assert!(serde_json::from_value::<QuestionReplyRequest>(serde_json::json!({})).is_err());
    assert!(
        serde_json::from_value::<QuestionReplyRequest>(serde_json::json!({"answers": ["yes"]}))
            .is_err()
    );
}

#[tokio::test]
async fn question_routes_enforce_registered_methods() {
    let app: axum::Router<()> = router();

    for (method, uri) in [
        (Method::POST, "/question"),
        (Method::GET, "/question/request/reply"),
        (Method::GET, "/question/request/reject"),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(uri)
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("route should respond");

        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED, "{uri}");
    }
}

#[tokio::test]
async fn question_reply_route_rejects_body_without_answers() {
    let response = router::<()>()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/question/request/reply")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .expect("request should build"),
        )
        .await
        .expect("route should respond");

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

fn question_info(label: &str) -> question::Info {
    question::Info {
        question: format!("{label}?"),
        header: "Confirm".to_string(),
        options: vec![question::OptionInfo {
            label: label.to_string(),
            description: "Proceed".to_string(),
            preview: None,
        }],
        multiple: None,
        custom: None,
    }
}

async fn pending_request_id(session_id: &str) -> String {
    loop {
        let Json(items) = question_list().await;
        if let Some(item) = items.iter().find(|item| item.session_id == session_id) {
            break item.id.clone();
        }
        sleep(Duration::from_millis(10)).await;
    }
}

#[tokio::test]
async fn question_list_and_reply_complete_pending_request() {
    let session_id = format!("session-question-handler-{}", uuid::Uuid::new_v4());
    let ask_task = tokio::spawn(question::ask(question::AskInput {
        session_id: session_id.clone(),
        questions: vec![question_info("Continue")],
        tool: None,
    }));

    let request_id = pending_request_id(&session_id).await;

    let Json(ok) = question_reply(
        Path(request_id),
        Json(QuestionReplyRequest { answers: vec![vec!["Yes".to_string()]] }),
    )
    .await
    .expect("reply should succeed");
    assert!(ok);

    let answers = ask_task.await.expect("ask task should join").expect("ask should resolve");
    assert_eq!(answers, vec![vec!["Yes".to_string()]]);
}

#[tokio::test]
async fn question_reply_accepts_empty_answers() {
    let session_id = format!("session-question-handler-empty-{}", uuid::Uuid::new_v4());
    let ask_task = tokio::spawn(question::ask(question::AskInput {
        session_id: session_id.clone(),
        questions: vec![question_info("Continue")],
        tool: None,
    }));

    let request_id = pending_request_id(&session_id).await;
    let Json(ok) = question_reply(Path(request_id), Json(QuestionReplyRequest { answers: vec![] }))
        .await
        .expect("empty reply should succeed");
    assert!(ok);

    let answers = ask_task.await.expect("ask task should join").expect("ask should resolve");
    assert!(answers.is_empty());
}

#[tokio::test]
async fn question_reply_unknown_request_is_idempotent() {
    let Json(ok) = question_reply(
        Path("missing-question".to_string()),
        Json(QuestionReplyRequest { answers: vec![vec!["ignored".to_string()]] }),
    )
    .await
    .expect("unknown reply should still succeed");

    assert!(ok);
}

#[tokio::test]
async fn question_reject_pending_request_resolves_ask_with_rejected_error() {
    let session_id = format!("session-question-handler-reject-{}", uuid::Uuid::new_v4());
    let ask_task = tokio::spawn(question::ask(question::AskInput {
        session_id: session_id.clone(),
        questions: vec![question_info("Continue")],
        tool: None,
    }));

    let request_id = pending_request_id(&session_id).await;
    let Json(ok) = question_reject(Path(request_id)).await.expect("reject should succeed");
    assert!(ok);

    let err = ask_task.await.expect("ask task should join").expect_err("ask should reject");
    assert!(matches!(err, question::Error::Rejected(_)));
}

#[tokio::test]
async fn question_reject_unknown_request_is_idempotent() {
    let Json(ok) = question_reject(Path("missing-question".to_string()))
        .await
        .expect("unknown reject should still succeed");

    assert!(ok);
}
