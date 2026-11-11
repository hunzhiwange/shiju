use super::*;
use axum::http::StatusCode;
use serde_json::json;
use vw_api_types::id::TenantScope;

const TOKEN: &str = "sk-voice-fixture-06-private-token";
struct Server {
    url: String,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}
async fn server() -> Server {
    let owner = TenantScope::new("voice06", "owner").unwrap();
    crate::config::update_global_for_tenant(
        &owner,
        json!({"gateway":{"auth_enabled":true,"skeys":[{"skey":TOKEN,"name":"fixture"}]}}),
    )
    .await
    .unwrap();
    let info=serde_json::from_value(json!({"id":"chat06","slug":"fixture","projectID":"voice06-project","directory":"/fixture","title":"fixture","version":"1","time":{"created":1,"updated":1}})).unwrap();
    crate::session::ui_store::save_agent_session_scoped_for_tenant(
        &owner,
        &info,
        Some("voice06-project"),
    )
    .unwrap();
    let app = router::<()>()
        .layer(axum::middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    Server { url, task }
}
fn metadata() -> String {
    json!({"request_id":"req06","chat_session_id":"chat06","generation":1,"provider_id":"openai"})
        .to_string()
}
fn form(bytes: Vec<u8>) -> reqwest::multipart::Form {
    reqwest::multipart::Form::new().text("metadata", metadata()).part(
        "file",
        reqwest::multipart::Part::bytes(bytes)
            .file_name("voice.wav")
            .mime_str("audio/wav")
            .unwrap(),
    )
}
#[tokio::test]
async fn voice_task_06_transcription_rejects_cross_tenant_request() {
    let server = server().await;
    let response = reqwest::Client::new()
        .post(format!("{}/voice/transcriptions", server.url))
        .bearer_auth(TOKEN)
        .header("x-platform-id", "voice06")
        .header("x-company-id", "other")
        .multipart(form(vec![0; 44]))
        .send()
        .await
        .unwrap();
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "a tenant header must not grant access with another tenant's skey"
    );
}
#[tokio::test]
async fn voice_task_06_transcription_rejects_oversized_body() {
    let server = server().await;
    let response = reqwest::Client::new()
        .post(format!("{}/voice/transcriptions", server.url))
        .bearer_auth(TOKEN)
        .header("x-platform-id", "voice06")
        .header("x-company-id", "owner")
        .multipart(form(vec![0; 24_100_001]))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE, "actual upload must be bounded");
}

fn wav() -> Vec<u8> {
    let pcm = vec![0u8; 480];
    let size = pcm.len() as u32;
    let mut bytes = b"RIFF".to_vec();
    bytes.extend((36 + size).to_le_bytes());
    bytes.extend(b"WAVEfmt ");
    bytes.extend(16u32.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(24000u32.to_le_bytes());
    bytes.extend(48000u32.to_le_bytes());
    bytes.extend(2u16.to_le_bytes());
    bytes.extend(16u16.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend(size.to_le_bytes());
    bytes.extend(pcm);
    bytes
}
fn request(client: &reqwest::Client, server: &Server, company: &str) -> reqwest::RequestBuilder {
    client
        .post(format!("{}/voice/transcriptions", server.url))
        .bearer_auth(TOKEN)
        .header("x-platform-id", "voice06")
        .header("x-company-id", company)
}
#[tokio::test]
async fn voice_task_06_transcription_counts_chunked_body_without_content_length() {
    let server = server().await;
    let chunks = futures_util::stream::iter([Ok::<_, std::io::Error>(vec![0; 24_100_001])]);
    let response = request(&reqwest::Client::new(), &server, "owner")
        .header("content-type", "multipart/form-data; boundary=fixture")
        .body(reqwest::Body::wrap_stream(chunks))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}
#[tokio::test]
async fn voice_task_06_transcription_rejects_anonymous_and_disabled_auth() {
    let server = server().await;
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/voice/transcriptions", server.url))
        .header("x-platform-id", "voice06")
        .header("x-company-id", "owner")
        .multipart(form(wav()))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    crate::config::update_global_for_tenant(
        &TenantScope::new("voice06", "owner").unwrap(),
        json!({"gateway":{"auth_enabled":false}}),
    )
    .await
    .unwrap();
    let response = request(&client, &server, "owner").multipart(form(wav())).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
#[tokio::test]
async fn voice_task_06_transcription_rejects_media_and_unknown_fields() {
    let server = server().await;
    let client = reqwest::Client::new();
    let bad = reqwest::multipart::Form::new()
        .text("metadata", metadata())
        .part("file", reqwest::multipart::Part::bytes(wav()).mime_str("text/plain").unwrap());
    assert_eq!(
        request(&client, &server, "owner").multipart(bad).send().await.unwrap().status(),
        StatusCode::UNSUPPORTED_MEDIA_TYPE
    );
    let bad = reqwest::multipart::Form::new()
        .text("metadata", metadata())
        .text("url", "https://example.invalid/private");
    assert_eq!(
        request(&client, &server, "owner").multipart(bad).send().await.unwrap().status(),
        StatusCode::BAD_REQUEST
    );
}
#[tokio::test]
async fn voice_task_06_authenticated_foreign_session_is_not_found() {
    let server = server().await;
    let tenant = TenantScope::new("voice06", "other").unwrap();
    crate::config::update_global_for_tenant(&tenant,json!({"gateway":{"auth_enabled":true,"skeys":[{"skey":TOKEN,"name":"fixture"}]},"app_ui":{"voice":{"provider_id":"openai"}}})).await.unwrap();
    let response = request(&reqwest::Client::new(), &server, "other")
        .multipart(form(wav()))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
#[test]
fn voice_task_06_transcription_round_trip_deduplicates_and_cancels() {
    crate::env::provide_with(
        std::collections::HashMap::from([(
            "VOICE06_SERVICE_KEY".into(),
            "fixture-service-key".into(),
        )]),
        || {
            tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(
                async {
                    tokio::time::timeout(Duration::from_secs(20), round_trip()).await.unwrap()
                },
            );
        },
    );
}
async fn round_trip() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let calls = Arc::new(AtomicUsize::new(0));
    let count = calls.clone();
    let upstream = axum::Router::new().route(
        "/v1/audio/transcriptions",
        axum::routing::post(move |body: axum::body::Bytes| {
            let count = count.clone();
            async move {
                assert!(body.windows(4).any(|b| b == b"RIFF"));
                count.fetch_add(1, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(150)).await;
                axum::Json(json!({"text":"你好","languages":[{"code":"zh"}]}))
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let upstream_task = tokio::spawn(async move {
        axum::serve(listener, upstream).await.unwrap();
    });
    let server = server().await;
    let tenant = TenantScope::new("voice06", "owner").unwrap();
    crate::config::update_global_for_tenant(&tenant,json!({"app_ui":{"voice":{"dictation_enabled":true,"provider_id":"voice06-fixture"}},"providers":{"voice06-fixture":{"api":format!("http://{address}/v1"),"env":["VOICE06_SERVICE_KEY"],"options":{"baseURL":format!("http://{address}/v1")},"models":{"gpt-transcribe":{"status":"active"}}}}})).await.unwrap();
    let info=serde_json::from_value(json!({"id":"chat06","slug":"fixture","projectID":"voice06-project","directory":"/fixture","title":"fixture","version":"1","time":{"created":1,"updated":1}})).unwrap();
    crate::session::ui_store::save_agent_session_scoped_for_tenant(
        &tenant,
        &info,
        Some("voice06-project"),
    )
    .unwrap();
    let client = reqwest::Client::new();
    let form_for = |id: &str| {
        let metadata = json!({"request_id":id,"chat_session_id":"chat06","generation":1,"provider_id":"voice06-fixture"});
        reqwest::multipart::Form::new()
            .text("metadata", metadata.to_string())
            .part("file", reqwest::multipart::Part::bytes(wav()).mime_str("audio/wav").unwrap())
    };
    let (a, b) = tokio::join!(
        request(&client, &server, "owner").multipart(form_for("success")).send(),
        request(&client, &server, "owner").multipart(form_for("success")).send()
    );
    for response in [a.unwrap(), b.unwrap()] {
        assert_eq!(response.status(), StatusCode::OK);
        let response: TranscriptionResponse = response.json().await.unwrap();
        assert_eq!(response.text, "你好");
        assert_eq!(response.generation.get(), 1);
    }
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let response =
        request(&client, &server, "owner").multipart(form_for("success")).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let pending = request(&client, &server, "owner").multipart(form_for("cancelled")).send();
    let cancel = async {
        while calls.load(Ordering::SeqCst) < 2 {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        let response = client
            .delete(format!("{}/voice/transcriptions/cancelled", server.url))
            .bearer_auth(TOKEN)
            .header("x-platform-id", "voice06")
            .header("x-company-id", "owner")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    };
    let (response, ()) = tokio::join!(pending, cancel);
    assert_eq!(response.unwrap().status(), StatusCode::CONFLICT);
    upstream_task.abort();
}
#[tokio::test]
async fn voice_task_07_config_requires_owner_and_excludes_credentials() {
    let tenant = TenantScope::new("voice07", "owner").unwrap();
    crate::config::update_global_for_tenant(&tenant, json!({"gateway":{"auth_enabled":true,"skeys":[{"skey":TOKEN,"name":"fixture"}]},"app_ui":{"voice":{"dictation_enabled":false}}})).await.unwrap();
    let app = router::<()>()
        .layer(axum::middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let server = Server { url, task };
    let client = reqwest::Client::new();
    let url = format!("{}/voice/config", server.url);
    let response = client
        .get(&url)
        .header("x-platform-id", "voice07")
        .header("x-company-id", "owner")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let response = client
        .get(&url)
        .bearer_auth(TOKEN)
        .header("x-platform-id", "voice07")
        .header("x-company-id", "other")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let response = client
        .get(&url)
        .bearer_auth(TOKEN)
        .header("x-platform-id", "voice07")
        .header("x-company-id", "owner")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "no-store");
    let bytes = response.bytes().await.unwrap();
    assert!(!std::str::from_utf8(&bytes).unwrap().contains(TOKEN));
    let config: vw_config_types::voice::VoiceConfig = serde_json::from_slice(&bytes).unwrap();
    assert!(!config.dictation_enabled);
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(value.get("gateway").is_none());
    assert!(value.get("model_providers").is_none());
}
