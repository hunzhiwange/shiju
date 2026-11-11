use super::*;
use axum::{Router, http::StatusCode, response::IntoResponse, routing::post};

async fn server() -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let app = Router::new().route(
        "/upload",
        post(|request: Request| async {
            match read_upload(request).await {
                Ok((metadata, bytes)) => {
                    axum::Json(serde_json::json!({"request_id":metadata.request_id,"bytes":bytes}))
                        .into_response()
                }
                Err(code) => StatusCode::from_u16(code.http_status()).unwrap().into_response(),
            }
        }),
    );
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (url, task)
}
fn metadata() -> String {
    serde_json::json!({"request_id":"req06","chat_session_id":"chat06","generation":1,"provider_id":"openai"}).to_string()
}
#[tokio::test]
async fn voice_task_06_upload_rejects_oversized_chunked_stream() {
    let (url, task) = server().await;
    let chunks = futures_util::stream::iter([Ok::<_, std::io::Error>(vec![0; 24_100_001])]);
    let response = reqwest::Client::new()
        .post(format!("{url}/upload"))
        .header("content-type", "multipart/form-data; boundary=fixture")
        .body(reqwest::Body::wrap_stream(chunks))
        .send()
        .await
        .unwrap();
    task.abort();
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}
#[tokio::test]
async fn voice_task_06_upload_preserves_bytes_and_rejects_paths_media_duplicates_and_metadata_limit()
 {
    let (url, task) = server().await;
    let client = reqwest::Client::new();
    let form = || {
        reqwest::multipart::Form::new().text("metadata", metadata()).part(
            "file",
            reqwest::multipart::Part::bytes(vec![1, 2, 3, 4]).mime_str("audio/wav").unwrap(),
        )
    };
    let response = client.post(format!("{url}/upload")).multipart(form()).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.json::<serde_json::Value>().await.unwrap(),
        serde_json::json!({"request_id":"req06","bytes":[1,2,3,4]})
    );
    for (body, status) in [
        (form().text("path", "/private/secret"), StatusCode::BAD_REQUEST),
        (form().text("metadata", metadata()), StatusCode::BAD_REQUEST),
        (
            reqwest::multipart::Form::new().text("metadata", "x".repeat(MAX_METADATA_BYTES + 1)),
            StatusCode::PAYLOAD_TOO_LARGE,
        ),
        (
            reqwest::multipart::Form::new().text("metadata", metadata()).part(
                "file",
                reqwest::multipart::Part::bytes(vec![1]).mime_str("text/plain").unwrap(),
            ),
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
        ),
        (form().text("url", "https://example.invalid"), StatusCode::BAD_REQUEST),
    ] {
        assert_eq!(
            client.post(format!("{url}/upload")).multipart(body).send().await.unwrap().status(),
            status
        );
    }
    task.abort();
}
