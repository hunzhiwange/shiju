use super::*;
use crate::{GatewayAuth, GatewayEndpoint};
use serde_json::json;
use std::{
    io::{Read, Write},
    net::TcpListener,
};
use vw_api_types::id::TenantScope;
fn metadata() -> TranscriptionRequest {
    serde_json::from_value(json!({"request_id":"req/06?","chat_session_id":"chat06","generation":1,"provider_id":"openai"})).unwrap()
}
fn fixture(status: u16, body: String) -> (GatewayClient, std::thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = GatewayEndpoint::new("127.0.0.1", listener.local_addr().unwrap().port())
        .with_auth(GatewayAuth { skey: Some("fixture-skey".into()) })
        .with_tenant_scope(TenantScope::new("p06", "c06").unwrap());
    let task = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
        let mut bytes = Vec::new();
        loop {
            let mut buf = [0; 4096];
            let count = stream.read(&mut buf).unwrap();
            if count == 0 {
                break;
            }
            bytes.extend_from_slice(&buf[..count]);
            if let Some(end) = bytes.windows(4).position(|b| b == b"\r\n\r\n") {
                let header = String::from_utf8_lossy(&bytes[..end]);
                let length: usize = header
                    .lines()
                    .find_map(|l| {
                        l.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .map(|v| v.trim().parse().unwrap())
                    })
                    .unwrap_or(0);
                if bytes.len() >= end + 4 + length {
                    break;
                }
            }
        }
        write!(stream,"HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
        String::from_utf8(bytes).unwrap()
    });
    (GatewayClient::new(endpoint).unwrap(), task)
}
#[tokio::test]
async fn voice_task_06_client_sends_authenticated_multipart_and_correlates_result() {
    let body=json!({"request_id":"req/06?","chat_session_id":"chat06","generation":1,"text":"你好","detected_languages":["zh"]}).to_string();
    let (client, task) = fixture(200, body);
    assert_eq!(
        client.voice_transcribe(&metadata(), b"RIFF-fixture".to_vec()).await.unwrap().text,
        "你好"
    );
    let request = task.join().unwrap();
    assert!(request.starts_with("POST /api/voice/transcriptions HTTP/1.1"));
    for expected in [
        "authorization: Bearer fixture-skey",
        "x-platform-id: p06",
        "x-company-id: c06",
        "name=\"metadata\"",
        "name=\"file\"",
        "audio/wav",
        "RIFF-fixture",
        "\"generation\":1",
    ] {
        assert!(request.contains(expected), "{expected}");
    }
}
#[tokio::test]
async fn voice_task_06_client_rejects_mismatched_generation() {
    let body=json!({"request_id":"req/06?","chat_session_id":"chat06","generation":2,"text":"stale","detected_languages":[]}).to_string();
    let (client, task) = fixture(200, body);
    assert_eq!(
        client.voice_transcribe(&metadata(), vec![1]).await,
        Err(VoiceApiError::InvalidResponse)
    );
    task.join().unwrap();
}
#[tokio::test]
async fn voice_task_06_client_redacts_gateway_error_and_encodes_cancel_id() {
    let mut error =
        vw_api_types::voice::VoiceError::new(VoiceErrorCode::ProviderAuth, metadata().request_id);
    error.message = "fixture-private-secret".into();
    let (client, task) =
        fixture(502, serde_json::to_string(&VoiceErrorResponse { error }).unwrap());
    let error = client.voice_transcribe(&metadata(), vec![1]).await.unwrap_err();
    assert_eq!(error, VoiceApiError::Service(VoiceErrorCode::ProviderAuth));
    assert!(!error.to_string().contains("private"));
    task.join().unwrap();
    let (client, task) = fixture(204, String::new());
    client.voice_cancel_transcription(&metadata().request_id).await.unwrap();
    assert!(
        task.join().unwrap().starts_with("DELETE /api/voice/transcriptions/req%2F06%3F HTTP/1.1")
    );
}
#[tokio::test]
async fn voice_task_06_client_rejects_oversized_audio_before_network() {
    let client = GatewayClient::new(GatewayEndpoint::new("127.0.0.1", 1)).unwrap();
    assert_eq!(
        client.voice_transcribe(&metadata(), vec![0; MAX_WAV_BYTES + 1]).await,
        Err(VoiceApiError::Service(VoiceErrorCode::PayloadTooLarge))
    );
}

#[tokio::test]
async fn voice_task_06_client_drop_notifies_gateway_cancellation() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = GatewayEndpoint::new("127.0.0.1", listener.local_addr().unwrap().port())
        .with_auth(GatewayAuth { skey: Some("fixture-skey".into()) })
        .with_tenant_scope(TenantScope::new("p06", "c06").unwrap());
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let (cancel_tx, cancel_rx) = std::sync::mpsc::channel();
    let task = std::thread::spawn(move || {
        let accept = || {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            loop {
                match listener.accept() {
                    Ok((socket, _)) => return socket,
                    Err(error)
                        if error.kind() == std::io::ErrorKind::WouldBlock
                            && std::time::Instant::now() < deadline =>
                    {
                        std::thread::sleep(std::time::Duration::from_millis(5))
                    }
                    Err(error) => panic!("accept fixture: {error}"),
                }
            }
        };
        let mut upload = accept();
        upload.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
        let mut bytes = [0; 8192];
        let count = upload.read(&mut bytes).unwrap();
        assert!(
            String::from_utf8_lossy(&bytes[..count]).starts_with("POST /api/voice/transcriptions ")
        );
        ready_tx.send(()).unwrap();
        let mut cancel = accept();
        cancel.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
        let mut bytes = [0; 8192];
        let count = cancel.read(&mut bytes).unwrap();
        let request = String::from_utf8_lossy(&bytes[..count]).to_string();
        write!(cancel, "HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n").unwrap();
        cancel_tx.send(request).unwrap();
    });
    let client = GatewayClient::new(endpoint).unwrap();
    let pending =
        tokio::spawn(async move { client.voice_transcribe(&metadata(), vec![1; 64]).await });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        if ready_rx.try_recv().is_ok() {
            break;
        }
        assert!(std::time::Instant::now() < deadline);
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    pending.abort();
    let _ = pending.await;
    let request = loop {
        if let Ok(request) = cancel_rx.try_recv() {
            break request;
        }
        assert!(std::time::Instant::now() < deadline);
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    };
    assert!(request.starts_with("DELETE /api/voice/transcriptions/req%2F06%3F HTTP/1.1"));
    assert!(request.contains("authorization: Bearer fixture-skey"));
    assert!(request.contains("x-company-id: c06"));
    task.join().unwrap();
}

#[tokio::test]
async fn voice_task_06_client_refuses_remote_plaintext_recordings() {
    let client = GatewayClient::new(GatewayEndpoint::new("example.invalid", 80)).unwrap();
    let expected = VoiceApiError::Service(VoiceErrorCode::UnsupportedCapability);
    assert_eq!(client.voice_transcribe(&metadata(), vec![1]).await, Err(expected.clone()));
    assert_eq!(client.voice_cancel_transcription(&metadata().request_id).await, Err(expected));
}
#[tokio::test]
async fn voice_task_07_client_fetches_scoped_voice_config_only() {
    let (client, task) =
        fixture(200, json!({"dictation_enabled":true,"provider_id":"voice"}).to_string());
    let config: serde_json::Value = client.voice_config().await.unwrap();
    assert_eq!(config["provider_id"], "voice");
    let request = task.join().unwrap();
    assert!(request.starts_with("GET /api/voice/config HTTP/1.1"));
    assert!(request.contains("authorization: Bearer fixture-skey"));
    assert!(request.contains("x-platform-id: p06"));
    assert!(request.contains("x-company-id: c06"));
}
#[tokio::test]
async fn voice_task_07_client_bounds_config_and_redacts_errors() {
    let (client, task) = fixture(200, " ".repeat(65 * 1024));
    assert_eq!(
        client.voice_config::<serde_json::Value>().await.unwrap_err(),
        VoiceApiError::InvalidResponse
    );
    task.join().unwrap();
    let (client, task) = fixture(500, "sensitive-upstream-payload".into());
    let error = client.voice_config::<serde_json::Value>().await.unwrap_err();
    assert_eq!(error, VoiceApiError::Transport);
    assert!(!error.to_string().contains("sensitive"));
    task.join().unwrap();
}

#[tokio::test]
async fn voice_task_13_client_patches_only_voice_fields_with_tenant_auth() {
    let (client, task) = fixture(204, String::new());
    client.voice_settings_patch(&json!({"voice":"cedar", "provider_id":null})).await.unwrap();
    let request = task.join().unwrap();
    assert!(request.starts_with("PATCH /api/voice/config HTTP/1.1"));
    assert!(request.contains("authorization: Bearer fixture-skey"));
    assert!(request.contains("x-platform-id: p06"));
    assert!(request.contains("x-company-id: c06"));
    let body: serde_json::Value =
        serde_json::from_str(request.split("\r\n\r\n").nth(1).unwrap()).unwrap();
    assert_eq!(body, json!({"voice":"cedar", "provider_id":null}));
}
