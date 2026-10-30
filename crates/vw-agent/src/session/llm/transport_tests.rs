//! Exercise interrupted HTTP responses through the real SDK and retry boundary.

use super::super::recovery_test_support::RecoveryConfig;
use super::stream_tests::test_input;
use super::*;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone, Copy, Debug)]
enum Interruption {
    BeforeHeaders,
    BeforeOutput,
    AfterOutput,
}

fn read_request(socket: &mut TcpStream) -> Vec<u8> {
    socket.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let mut request = Vec::new();
    loop {
        let mut bytes = [0; 4096];
        let size = socket.read(&mut bytes).expect("read fixture request");
        assert_ne!(size, 0);
        request.extend_from_slice(&bytes[..size]);
        let Some(end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") else {
            continue;
        };
        let headers = String::from_utf8_lossy(&request[..end]);
        let length = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length").then(|| value.trim().parse().unwrap())
            })
            .unwrap_or(0usize);
        if request.len() >= end + 4 + length {
            return request;
        }
    }
}

async fn interrupted_response(interruption: Interruption) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let count = Arc::new(AtomicUsize::new(0));
    let server_stop = stop.clone();
    let server_count = count.clone();
    let (output_tx, output_rx) = std::sync::mpsc::channel();
    let server = std::thread::spawn(move || {
        for socket in listener.incoming() {
            let mut socket = socket.unwrap();
            if server_stop.load(Ordering::SeqCst) {
                break;
            }
            let request = read_request(&mut socket);
            if request.starts_with(b"GET ") {
                let body = r#"{"data":[{"id":"original"}]}"#;
                write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
                continue;
            }
            assert!(request.starts_with(b"POST /v1/chat/completions "));
            let first = server_count.fetch_add(1, Ordering::SeqCst) == 0;
            if first && matches!(interruption, Interruption::BeforeHeaders) {
                continue;
            }
            let chunk = json!({"id":"fixture","object":"chat.completion.chunk","created":0,
                "model":"original","choices":[{"index":0,
                "delta":{"content":if first {"partial"} else {"recovered"}},"finish_reason":null}]});
            let body = format!("data: {chunk}\n\n");
            if first {
                // Advertise more bytes than are sent, reproducing reqwest's body decode error.
                write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len() + 100).unwrap();
                if matches!(interruption, Interruption::AfterOutput) {
                    socket.write_all(body.as_bytes()).unwrap();
                    socket.flush().unwrap();
                    output_rx
                        .recv_timeout(Duration::from_secs(5))
                        .expect("partial output observed");
                }
                continue;
            }
            let finish = json!({"id":"fixture","object":"chat.completion.chunk","created":0,
                "model":"original","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]});
            let body = format!("{body}data: {finish}\n\ndata: [DONE]\n\n");
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
        }
    });
    let fixture = RecoveryConfig::new(&format!("http://{address}/v1"), Some("fixture-key")).await;
    let mut input = test_input("model-recovery");
    input.model = provider::get_model("model-recovery", "original").await.unwrap();
    input.agent.options.insert("max_retries".into(), json!(1));
    let observed = Arc::new(Mutex::new((String::new(), 0)));
    let capture = observed.clone();
    let result = stream(input, move |event| {
        let mut observed = capture.lock().unwrap();
        match event {
            StreamEvent::Delta(delta) => {
                if delta == "partial" {
                    output_tx.send(()).unwrap();
                }
                observed.0.push_str(&delta);
            }
            StreamEvent::Done { .. } => observed.1 += 1,
            _ => {}
        }
    })
    .await;
    stop.store(true, Ordering::SeqCst);
    let _ = TcpStream::connect(address);
    server.join().expect("fixture server stopped");
    drop(fixture);
    let (text, done) = observed.lock().unwrap().clone();

    if matches!(interruption, Interruption::AfterOutput) {
        assert_eq!(count.load(Ordering::SeqCst), 1, "partial response must not be replayed");
        assert_eq!(text, "partial");
        assert_eq!(done, 0);
        assert!(matches!(result, Err(Error::Api(message::AssistantError::APIError {
            is_retryable: false, metadata: Some(metadata), ..
        })) if metadata.get("replay_safe").map(String::as_str) == Some("false")));
    } else {
        assert!(result.is_ok(), "{interruption:?}: {result:?}");
        assert_eq!(count.load(Ordering::SeqCst), 2, "one bounded retry");
        assert_eq!(text, "recovered");
        assert_eq!(done, 1);
    }
}

#[test]
fn model_transport_interruption_retries_only_before_output() {
    crate::env::provide_with(
        HashMap::from([("VW_MODEL_RECOVERY_TEST_KEY".into(), "fixture-key".into())]),
        || {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                let _env = crate::test_support::global_async_env_lock().lock().await;
                for interruption in [
                    Interruption::BeforeHeaders,
                    Interruption::BeforeOutput,
                    Interruption::AfterOutput,
                ] {
                    interrupted_response(interruption).await;
                }
            });
        },
    );
}
