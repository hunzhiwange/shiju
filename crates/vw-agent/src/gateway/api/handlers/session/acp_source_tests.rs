//! Production source handlers over a real loopback HTTP boundary and isolated database.
use axum::{
    Router,
    routing::{get, post, put},
};
use serde_json::json;

#[tokio::test]
async fn acp_model_source_ui_http_persists_and_rejects_without_local_fallback() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        Some("fixture-key"),
    )
    .await;
    let workspace = tempfile::tempdir().unwrap();
    let directory = workspace.path().to_string_lossy().into_owned();
    let router = Router::new()
        .route("/session", post(super::session_ops::ui_session_create))
        .route("/session/{session_id}/acp-model-source", put(super::acp_options::set_source))
        .route("/session/{session_id}/acp-options", get(super::acp_options::get));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let http = reqwest::Client::new();
    let session: serde_json::Value = http
        .post(format!("http://{address}/session"))
        .query(&[("directory", &directory)])
        .json(&json!({}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    let id = session["id"].as_str().unwrap();
    let url = format!("http://{address}/session/{id}/acp-model-source");
    let selected = http
        .put(&url)
        .query(&[("directory", &directory)])
        .json(&json!({"agent":"qoder","selection":{"source":"cli_local"}}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json::<vw_api_types::acp_session::AcpSessionOptions>()
        .await
        .unwrap();
    assert_eq!(selected.acp_model_source, vw_api_types::acp_model_source::AcpModelSource::CliLocal);
    assert!(!selected.connected, "selection does not start a CLI");
    let rejected = http.put(&url).query(&[("directory", &directory)])
        .json(&json!({"agent":"qoder","selection":{"source":"session_model","model":"model-recovery/original"}}))
        .send().await.unwrap();
    assert_eq!(rejected.status(), reqwest::StatusCode::BAD_REQUEST);
    let saved = http
        .get(format!("http://{address}/session/{id}/acp-options"))
        .query(&[("directory", &directory)])
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json::<vw_api_types::acp_session::AcpSessionOptions>()
        .await
        .unwrap();
    assert_eq!(saved.selected_model, selected.selected_model);
    assert_eq!(saved.acp_model_source, selected.acp_model_source);
    crate::config::update(json!({"providers":{"model-recovery":{"models":{"original":{
        "status":"active", "tool_call":true, "temperature":true
    }}}}}))
    .await
    .unwrap();
    crate::provider::provider::invalidate_cache().await;
    let managed_response = http.put(&url).query(&[("directory", &directory)])
        .json(&json!({"agent":"claude","selection":{"source":"session_model","model":"model-recovery/original"}}))
        .send().await.unwrap();
    let status = managed_response.status();
    let body = managed_response.text().await.unwrap();
    assert!(status.is_success(), "{status}: {body}");
    let managed: vw_api_types::acp_session::AcpSessionOptions =
        serde_json::from_str(&body).unwrap();
    assert_eq!(
        managed.acp_model_source,
        vw_api_types::acp_model_source::AcpModelSource::SessionModel
    );
    assert_eq!(
        managed.selected_model.as_ref().unwrap().model.as_deref(),
        Some("model-recovery/original")
    );
    let disabled = http.put(&url).query(&[("directory", &directory)])
        .json(&json!({"agent":"claude","selection":{"source":"session_model","model":"model-recovery/disabled"}}))
        .send().await.unwrap();
    assert_eq!(disabled.status(), reqwest::StatusCode::BAD_REQUEST);
    let recovered = http
        .get(format!("http://{address}/session/{id}/acp-options"))
        .query(&[("directory", &directory)])
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json::<vw_api_types::acp_session::AcpSessionOptions>()
        .await
        .unwrap();
    assert_eq!(recovered.selected_model, managed.selected_model);
    server.abort();
    let _ = server.await;
}

#[tokio::test]
async fn acp_public_permission_options_reject_direct_put_and_chat_over_http() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        Some("fixture-key"),
    )
    .await;
    let workspace = tempfile::tempdir().unwrap();
    let router = Router::new()
        .route("/session", post(super::session_ops::ui_session_create))
        .route("/session/{session_id}/acp-options", put(super::acp_options::set))
        .route("/chat/stream", post(super::stream::chat_stream));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let http = reqwest::Client::new();
    let dir = [("directory", workspace.path().to_str().unwrap())];
    let session: serde_json::Value = http
        .post(format!("http://{address}/session"))
        .query(&dir)
        .json(&json!({}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    let id = session["id"].as_str().unwrap();
    for selection in [
        json!({"kind":{"kind":"mode"}, "value":"bypassPermissions"}),
        json!({"kind":{"kind":"config","id":"edit_approval_policy"}, "value":"never"}),
    ] {
        let response = http
            .put(format!("http://{address}/session/{id}/acp-options"))
            .query(&dir)
            .json(&selection)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST);
        assert!(response.text().await.unwrap().contains("full_access"));
    }
    for (options, expected) in [
        (json!({"acp_session_mode":"bypassPermissions"}), "acp_session_mode"),
        (json!({"acp_session_config":{"edit_approval_policy":"never"}}), "acp_session_config"),
        (json!({"acp_session_config":{"custom-control":"bypass"}}), "acp_session_config"),
    ] {
        let response = http
            .post(format!("http://{address}/chat/stream"))
            .query(&dir)
            .json(&json!({
                "session_id": id, "acp_agent":"claude", "messages":[],
                "reasoning_effort":"high", "options":options
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST);
        assert!(response.text().await.unwrap().contains(expected));
    }
    server.abort();
}
