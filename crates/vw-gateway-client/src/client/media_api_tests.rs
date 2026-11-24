use super::*;
use crate::client::test_support;
use gateway_endpoint::{GatewayAuth, GatewayEndpoint};
use serde_json::json;
use std::{
    io::{BufRead, BufReader, Write},
    net::TcpListener,
    thread,
    time::Duration,
};
use vw_api_transport::id::{TenantScope, UserId};

fn session() -> MediaSessionQuery {
    MediaSessionQuery { session_id: "abc/def?ghi".into() }
}

fn request() -> MediaSubmit {
    MediaSubmit {
        request_id: MediaJobId::new_v4(),
        session_id: session().session_id,
        operation: MediaOperation::VideoGenerate,
        deployment_id: "direct".into(),
        model_id: "video-model".into(),
        text: Some("Moonlight".into()),
        inputs: vec![MediaInput::Asset { asset_id: "authorized-input".into() }],
    }
}

fn job(request: &MediaSubmit, status: &str) -> serde_json::Value {
    json!({
        "id": request.request_id, "session_id": request.session_id, "operation": "video_generate",
        "created_at_ms": 50, "updated_at_ms": 51, "status": status,
    })
}

#[tokio::test]
async fn media_job_client_methods_auth_scopes_and_submit_id_match_the_wire_contract() {
    let request = request();
    let tenant = TenantScope::new("platform-fixture", "company-fixture").unwrap();
    let server = test_support::server_with_tenant_scope(
        vec![
            (
                200,
                json!({"models":[{"deployment_id":"direct","model_id":"video-model","display_name":"Video","operations":["video_generate"]}]}),
            ),
            (201, job(&request, "submitted")),
            (200, job(&request, "submission_unknown")),
            (200, job(&request, "canceled")),
        ],
        tenant,
    );
    let client = GatewayClient::new(
        server
            .client()
            .endpoint()
            .clone()
            .with_auth(GatewayAuth { skey: Some("fixture-token".into()) })
            .with_user_id(UserId::new("user-fixture").unwrap()),
    )
    .unwrap();
    assert_eq!(client.media_models(&session()).await.unwrap().models[0].model_id, "video-model");
    assert_eq!(client.media_submit(&request).await.unwrap().id, request.request_id);
    assert_eq!(
        client.media_job(&session(), request.request_id).await.unwrap().state,
        MediaJobState::SubmissionUnknown
    );
    assert_eq!(
        client.media_cancel(&session(), request.request_id).await.unwrap().state,
        MediaJobState::Canceled
    );

    let job_path = format!("/api/media/jobs/{}?session_id=abc%2Fdef%3Fghi", request.request_id);
    for (method, path) in [
        ("GET", "/api/media/models?session_id=abc%2Fdef%3Fghi"),
        ("POST", "/api/media/jobs"),
        ("GET", job_path.as_str()),
        ("DELETE", job_path.as_str()),
    ] {
        let recorded = server.take_request();
        assert_eq!(recorded.method, method);
        assert_eq!(recorded.path, path);
        assert_eq!(recorded.header("Authorization"), Some("Bearer fixture-token"));
        assert_eq!(recorded.header("X-Platform-ID"), Some("platform-fixture"));
        assert_eq!(recorded.header("X-Company-ID"), Some("company-fixture"));
        assert_eq!(recorded.header("X-User-ID"), Some("user-fixture"));
        if method == "POST" {
            assert_eq!(
                recorded.body,
                json!({
                    "request_id": request.request_id,
                    "session_id": "abc/def?ghi",
                    "operation": "video_generate",
                    "deployment_id": "direct",
                    "model_id": "video-model",
                    "text": "Moonlight",
                    "inputs": [{"type": "asset", "asset_id":"authorized-input"}],
                })
            );
        }
    }
    server.join();
}

#[tokio::test]
async fn media_job_client_maps_authorization_and_conflict_without_leaking_gateway_messages() {
    let request = request();
    let server = test_support::server(vec![
        (401, json!({"message":"internal bearer token"})),
        (409, json!({"error":{"code":"conflict","message":"upstream credentials"}})),
        (404, json!({"error":{"code":"not_found"}})),
    ]);
    let client = server.client();
    assert_eq!(
        client.media_models(&session()).await,
        Err(MediaApiError::Service(Code::Unauthorized))
    );
    assert_eq!(client.media_submit(&request).await, Err(MediaApiError::Service(Code::Conflict)));
    assert_eq!(
        client.media_job(&session(), request.request_id).await,
        Err(MediaApiError::Service(Code::NotFound))
    );
    server.join();
}

#[tokio::test]
async fn media_job_client_correlates_lost_submit_response_with_known_job_id() {
    let request = request();
    let server = test_support::server(vec![
        (502, json!({"error":{"code":"upstream_failed"}})),
        (200, job(&request, "running")),
    ]);
    let client = server.client();
    let err = client.media_submit(&request).await.unwrap_err();
    let MediaApiError::SubmissionUnknown { job_id } = err else {
        panic!("ambiguous POST must preserve the known ID");
    };
    assert_eq!(job_id, request.request_id);
    assert_eq!(
        client.media_job(&session(), job_id).await.unwrap().state,
        MediaJobState::Running { progress_percent: None }
    );
    assert_eq!(server.take_request().method, "POST");
    assert_eq!(server.take_request().method, "GET");
    server.join();
}

fn asset_fixture(mime: &str, chunks: &[&[u8]]) -> (GatewayClient, thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let client = GatewayClient::new(GatewayEndpoint::new(
        "127.0.0.1",
        listener.local_addr().unwrap().port(),
    ))
    .unwrap();
    let mime = mime.to_owned();
    let chunks: Vec<Vec<u8>> = chunks.iter().map(|chunk| chunk.to_vec()).collect();
    let handle = thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let request_line = {
            let mut reader = BufReader::new(&mut socket);
            let mut request_line = String::new();
            reader.read_line(&mut request_line).unwrap();
            loop {
                let mut header = String::new();
                assert_ne!(reader.read_line(&mut header).unwrap(), 0);
                if header == "\r\n" {
                    break;
                }
            }
            request_line
        };
        write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: {mime}\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n").unwrap();
        let mut response = Vec::new();
        for chunk in chunks {
            write!(response, "{:x}\r\n", chunk.len()).unwrap();
            response.extend_from_slice(&chunk);
            response.extend_from_slice(b"\r\n");
        }
        response.extend_from_slice(b"0\r\n\r\n");
        // Rejecting the response MIME can close the socket.
        let _ = socket.write_all(&response);
        request_line
    });
    (client, handle)
}

fn video(byte_len: u64) -> MediaAsset {
    MediaAsset {
        id: "video-1".into(),
        mime: "video/mp4".into(),
        byte_len,
        width: Some(1024),
        height: Some(768),
        duration_ms: Some(8000),
        source: MediaAssetSource::Stream,
    }
}

#[tokio::test]
async fn media_job_client_streams_binary_asset_using_authorized_job_and_asset_identifiers() {
    let (client, fixture) = asset_fixture("video/mp4", &[b"abc", b"\0\xff\x01"]);
    let id = request().request_id;
    let mut stream = client.media_asset(&session(), id, &video(6)).await.unwrap();
    let mut bytes = Vec::new();
    while let Some(chunk) = stream.next().await {
        bytes.extend(chunk.unwrap());
    }
    assert_eq!(bytes, b"abc\0\xff\x01");
    assert_eq!(
        fixture.join().unwrap(),
        format!("GET /api/media/jobs/{id}/assets/video-1?session_id=abc%2Fdef%3Fghi HTTP/1.1\r\n")
    );
}

#[tokio::test]
async fn media_job_client_rejects_mismatched_mime_and_short_or_oversized_streams() {
    let id = request().request_id;
    let (client, fixture) = asset_fixture("text/html", &[b"abc"]);
    assert!(matches!(
        client.media_asset(&session(), id, &video(3)).await,
        Err(MediaApiError::InvalidResponse)
    ));
    fixture.join().unwrap();

    for expected_length in [2, 4] {
        let (client, fixture) = asset_fixture("video/mp4", &[b"abc"]);
        let mut stream = client.media_asset(&session(), id, &video(expected_length)).await.unwrap();
        if expected_length > 3 {
            assert_eq!(stream.next().await, Some(Ok(b"abc".to_vec())));
        }
        assert_eq!(stream.next().await, Some(Err(MediaApiError::InvalidResponse)));
        assert!(stream.next().await.is_none());
        fixture.join().unwrap();
    }
}
