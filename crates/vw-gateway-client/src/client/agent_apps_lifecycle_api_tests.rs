use super::{AgentAppsLifecycleError, test_support};
use serde_json::json;
use vw_api_types::{agent_apps::*, id::TaskId};

#[tokio::test]
async fn cc_apps_lifecycle_sdk_roundtrip_preserves_ids_and_recovers_by_reading() {
    let request = json!({"appId":"codex","action":"install","target":{"kind":"environment","environment":{"kind":"macos"}}});
    // Use the shared environment wire shape, not a client-side installation recipe.
    let request = serde_json::from_value::<AppActionPreviewRequest>(request).unwrap();
    let preview = json!({"previewId":"preview-1","request":request,"source":"native","confirmationRequired":true,"summary":"Review","displayCommand":null});
    let status = json!({"operationId":"op /?","status":"running","phase":"executing"});
    let server = test_support::server(vec![
        (200, preview),
        (202, json!({"operationId":"op /?","status":"queued"})),
        (503, json!({"code":"disconnected"})),
        (200, status.clone()),
        (200, status),
        (200, json!({"operationId":"op /?","status":"cancelled","phase":"terminal"})),
    ]);
    let client = super::GatewayClient::new(
        server
            .client()
            .endpoint()
            .clone()
            .with_auth(crate::GatewayAuth { skey: Some("lifecycle-token".into()) }),
    )
    .unwrap();
    let preview = client.agent_apps_action_preview("host /", &request).await.unwrap();
    let submit = AppActionSubmitRequest { preview_id: preview.preview_id, confirmed: true };
    let accepted = client.agent_apps_action_submit("host /", &submit).await.unwrap();
    let id = accepted.operation_id;
    assert_eq!(id, TaskId::from("op /?"));
    assert!(matches!(
        client.agent_apps_operation_events("host /", &id).await,
        Err(AgentAppsLifecycleError::Http { status: 503, .. })
    ));
    assert_eq!(client.agent_apps_operation("host /", &id).await.unwrap().operation_id, id);
    assert_eq!(client.agent_apps_operation_events("host /", &id).await.unwrap().operation_id, id);
    let cancelled = client.agent_apps_operation_cancel("host /", &id).await.unwrap();
    assert_eq!(cancelled.operation_id, id);
    assert_eq!(cancelled.phase, AppOperationPhase::Terminal);
    for (method, path) in [
        ("POST", "actions/preview"),
        ("POST", "actions"),
        ("GET", "operations/op%20%2F%3F/events"),
        ("GET", "operations/op%20%2F%3F"),
        ("GET", "operations/op%20%2F%3F/events"),
        ("DELETE", "operations/op%20%2F%3F"),
    ] {
        let recorded = server.take_request();
        assert_eq!(recorded.method, method);
        assert_eq!(recorded.path, format!("/v1/agent-apps/{path}?targetHostId=host+%2F"));
        if path == "actions" {
            assert_eq!(recorded.body, serde_json::to_value(&submit).unwrap());
        }
        if method != "POST" {
            assert_eq!(recorded.body, serde_json::Value::Null);
        }
        assert!(recorded.header("x-platform-id").is_some());
        assert!(recorded.header("x-company-id").is_some());
        assert_eq!(recorded.header("authorization"), Some("Bearer lifecycle-token"));
    }
    server.join();
}

#[tokio::test]
async fn cc_apps_lifecycle_sdk_roundtrip_business_errors_are_not_replayed() {
    for (status, code) in [
        (409, "conflict"),
        (409, "plan_expired"),
        (422, "node_insufficient"),
        (307, "redirect"),
        (503, "unavailable"),
    ] {
        let body = json!({"code":code,"details":{"required":"22"}});
        let server = test_support::server(vec![(status, body.clone())]);
        let request =
            AppActionSubmitRequest { preview_id: AppActionPreviewId("p".into()), confirmed: true };
        assert_eq!(
            server.client().agent_apps_action_submit("host", &request).await.unwrap_err(),
            AgentAppsLifecycleError::Http { status, body }
        );
        assert_eq!(server.take_request().method, "POST");
        server.join();
    }
}

#[tokio::test]
async fn cc_apps_lifecycle_sdk_roundtrip_dropped_reply_and_redirect_never_repeat_write() {
    use std::io::{BufRead, Read, Write};
    use std::net::TcpListener;
    use std::time::{Duration, Instant};
    for redirect in [false, true] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = std::thread::spawn(move || {
            listener.set_nonblocking(true).unwrap();
            let deadline = Instant::now() + Duration::from_secs(3);
            let mut count = 0;
            let mut first_seen = None;
            while Instant::now() < deadline {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream.set_nonblocking(false).unwrap();
                        stream.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
                        let mut reader = std::io::BufReader::new(&mut stream);
                        let mut length = 0;
                        loop {
                            let mut line = String::new();
                            reader.read_line(&mut line).unwrap();
                            if line == "\r\n" {
                                break;
                            }
                            if let Some(value) =
                                line.to_ascii_lowercase().strip_prefix("content-length:")
                            {
                                length = value.trim().parse::<usize>().unwrap();
                            }
                        }
                        let mut body = vec![0; length];
                        reader.read_exact(&mut body).unwrap();
                        assert_eq!(
                            serde_json::from_slice::<serde_json::Value>(&body).unwrap()["previewId"],
                            "p"
                        );
                        count += 1;
                        first_seen.get_or_insert(Instant::now());
                        if redirect {
                            write!(stream,"HTTP/1.1 307 Temporary Redirect\r\nLocation: http://127.0.0.1:{port}/replayed\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
                        }
                        // Closing without a reply models an accepted write whose acknowledgement was lost.
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        if first_seen
                            .is_some_and(|at: Instant| at.elapsed() > Duration::from_millis(300))
                        {
                            break;
                        }
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("{error}"),
                }
            }
            count
        });
        let client =
            super::GatewayClient::new(crate::GatewayEndpoint::new("127.0.0.1", port)).unwrap();
        let result = client
            .agent_apps_action_submit(
                "h",
                &AppActionSubmitRequest {
                    preview_id: AppActionPreviewId("p".into()),
                    confirmed: true,
                },
            )
            .await;
        if redirect {
            assert_eq!(
                result.unwrap_err(),
                AgentAppsLifecycleError::Http { status: 307, body: json!("") }
            );
        } else {
            assert!(matches!(result, Err(AgentAppsLifecycleError::Transport(_))));
        }
        assert_eq!(handle.join().unwrap(), 1, "write must reach the server exactly once");
    }
}

#[tokio::test]
async fn cc_apps_lifecycle_sdk_roundtrip_invalid_and_empty_responses() {
    let server = test_support::server(vec![(200, json!({"unexpected":true}))]);
    assert!(matches!(
        server.client().agent_apps_operation("h", &TaskId::from("id")).await,
        Err(AgentAppsLifecycleError::Decode(_))
    ));
    server.join();
    let server = test_support::server_raw(vec![(409, String::new())]);
    assert_eq!(
        server.client().agent_apps_operation_cancel("h", &TaskId::from("id")).await.unwrap_err(),
        AgentAppsLifecycleError::Http { status: 409, body: json!("") }
    );
    server.join();
}
