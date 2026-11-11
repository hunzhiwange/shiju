use super::test_support;
use crate::{AggregationApiError, GatewayClient};
use serde_json::{Value, json};
use vw_api_types::agent_apps::{AppId, aggregation::*};

fn scope() -> AggregationQuery {
    AggregationQuery {
        target_host_id: "host /?中文".into(), project_id: "project /?中文".into()
    }
}
fn publish() -> AggregationPublishRequest {
    let s = scope();
    AggregationPublishRequest {
        target_host_id: s.target_host_id,
        project_id: s.project_id,
        expected_revision: "opaque-revision".into(),
        request_id: "caller-request".into(),
        preview_token: "server-preview".into(),
        confirmed: true,
    }
}
fn exit() -> AggregationExitRequest {
    let p = publish();
    AggregationExitRequest {
        target_host_id: p.target_host_id,
        project_id: p.project_id,
        expected_revision: p.expected_revision,
        request_id: p.request_id,
        confirmed: true,
    }
}
fn progress(status: &str, phase: &str, cancelled: bool, outcome: Value) -> Value {
    let mut value = json!({"operationId":"operation-51", "requestId":"caller-request", "status":status,"phase":phase,"completed":0,"total":1,"cancellationRequested":cancelled});
    if !outcome.is_null() {
        value["aggregationOutcome"] = outcome;
    }
    value
}
#[tokio::test]
async fn aggregation_t51_aggregation_publish_api_preserves_contract() {
    let initial = progress("running", "preparing", false, Value::Null);
    let failed = progress(
        "failed",
        "terminal",
        true,
        json!({"kind":"failed","status":502,"code":"recovery_failed","recovery":"pending"}),
    );
    let server = test_support::server(vec![
        (202, initial.clone()),
        (202, initial.clone()),
        (200, initial.clone()),
        (200, failed.clone()),
    ]);
    let client = GatewayClient::new(
        server
            .client()
            .endpoint()
            .clone()
            .with_auth(crate::GatewayAuth { skey: Some("t51-bearer".into()) }),
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(client.aggregation_publish(AppId::Codex, &publish()).await.unwrap())
            .unwrap(),
        initial
    );
    assert_eq!(
        serde_json::to_value(client.aggregation_exit(AppId::ClaudeDesktop, &exit()).await.unwrap())
            .unwrap(),
        initial
    );
    assert_eq!(
        serde_json::to_value(
            client.provider_aggregation_operation_get("operation-51", &scope()).await.unwrap()
        )
        .unwrap(),
        initial
    );
    assert_eq!(
        serde_json::to_value(
            client.provider_aggregation_operation_cancel("operation-51", &scope()).await.unwrap()
        )
        .unwrap(),
        failed
    );
    for (method, path, body) in [
        (
            "POST",
            "/v1/agent-apps/codex/aggregation/publish",
            serde_json::to_value(publish()).unwrap(),
        ),
        (
            "POST",
            "/v1/agent-apps/claude-desktop/aggregation/exit",
            serde_json::to_value(exit()).unwrap(),
        ),
        (
            "GET",
            "/v1/app-providers/operations/operation-51?targetHostId=host+%2F%3F%E4%B8%AD%E6%96%87&projectId=project+%2F%3F%E4%B8%AD%E6%96%87",
            Value::Null,
        ),
        (
            "DELETE",
            "/v1/app-providers/operations/operation-51?targetHostId=host+%2F%3F%E4%B8%AD%E6%96%87&projectId=project+%2F%3F%E4%B8%AD%E6%96%87",
            Value::Null,
        ),
    ] {
        let r = server.take_request();
        assert_eq!(r.method, method);
        assert_eq!(r.path, path);
        assert_eq!(r.body, body);
        assert_eq!(r.header("authorization"), Some("Bearer t51-bearer"));
        assert_eq!(
            r.header("x-company-id"),
            Some(client.endpoint().tenant_scope().company_id().as_ref())
        );
    }
    server.join();
}
#[tokio::test]
async fn aggregation_t51_errors_preserve_revision_and_recovery() {
    for (status, code) in [
        (401, "unauthenticated"),
        (403, "forbidden"),
        (409, "revision_conflict"),
        (409, "request_conflict"),
        (422, "unsupported_app"),
        (422, "unsupported_target"),
        (502, "recovery_failed"),
        (503, "target_unavailable"),
        (504, "target_timeout"),
    ] {
        let body = json!({"error":{"code":code,"message":"safe error","details":{"revision":"current-revision","recovery":"pending"}}});
        let server = test_support::server(vec![(status, body.clone()); 2]);
        let client = server.client();
        let expected = AggregationApiError::Http { status, body };
        assert_eq!(
            client.aggregation_publish(AppId::Claude, &publish()).await.unwrap_err(),
            expected
        );
        assert_eq!(client.aggregation_exit(AppId::Claude, &exit()).await.unwrap_err(), expected);
        server.join();
    }
}
#[tokio::test]
async fn aggregation_t51_ambiguous_writes_and_redirects_never_replay() {
    use std::{
        io::{BufRead, Read, Write},
        net::TcpListener,
        time::{Duration, Instant},
    };
    for (redirect, exit_action) in [(false, false), (false, true), (true, false), (true, true)] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = std::thread::spawn(move || {
            listener.set_nonblocking(true).unwrap();
            let deadline = Instant::now() + Duration::from_secs(4);
            let mut first = None;
            let mut count = 0;
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
                        let mut bytes = vec![0; length];
                        reader.read_exact(&mut bytes).unwrap();
                        assert_eq!(
                            serde_json::from_slice::<Value>(&bytes).unwrap(),
                            if exit_action {
                                serde_json::to_value(exit()).unwrap()
                            } else {
                                serde_json::to_value(publish()).unwrap()
                            }
                        );
                        count += 1;
                        first.get_or_insert(Instant::now());
                        if redirect {
                            write!(stream,"HTTP/1.1 307 Temporary Redirect\r\nLocation: http://127.0.0.1:{port}/replayed\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        if first
                            .is_some_and(|at: Instant| at.elapsed() > Duration::from_millis(350))
                        {
                            break;
                        }
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Err(e) => panic!("{e}"),
                }
            }
            count
        });
        let client = GatewayClient::new(crate::GatewayEndpoint::new("127.0.0.1", port)).unwrap();
        let result = if exit_action {
            client.aggregation_exit(AppId::Claude, &exit()).await
        } else {
            client.aggregation_publish(AppId::Claude, &publish()).await
        };
        if redirect {
            assert_eq!(
                result.unwrap_err(),
                AggregationApiError::Http { status: 307, body: json!("") }
            );
        } else {
            assert!(matches!(result, Err(AggregationApiError::Transport(_))));
        }
        assert_eq!(handle.join().unwrap(), 1);
    }
}

#[tokio::test]
async fn aggregation_t51_cancel_and_success_keep_authoritative_outcomes() {
    let state = json!({"revision":"new-revision","mode":"direct","attached":false,"defaultProviderId":"p","directProviderId":"p","members":[],"notices":[],"recovery":"restored"});
    let published =
        progress("completed", "terminal", false, json!({"kind":"published","state":state}));
    let exited = progress("completed", "terminal", false, json!({"kind":"exited","state":state}));
    let publishing = progress("running", "publishing", true, Value::Null);
    let cancelled = progress("cancelled", "terminal", true, Value::Null);
    let server = test_support::server(vec![
        (202, published.clone()),
        (202, exited.clone()),
        (200, publishing.clone()),
        (200, cancelled.clone()),
    ]);
    let client = server.client();
    assert_eq!(
        serde_json::to_value(client.aggregation_publish(AppId::Claude, &publish()).await.unwrap())
            .unwrap(),
        published
    );
    assert_eq!(
        serde_json::to_value(client.aggregation_exit(AppId::Codex, &exit()).await.unwrap())
            .unwrap(),
        exited
    );
    assert_eq!(
        serde_json::to_value(
            client.provider_aggregation_operation_cancel("operation-51", &scope()).await.unwrap()
        )
        .unwrap(),
        publishing
    );
    assert_eq!(
        serde_json::to_value(
            client.provider_aggregation_operation_get("operation-51", &scope()).await.unwrap()
        )
        .unwrap(),
        cancelled
    );
    server.join();
}

#[tokio::test]
async fn aggregation_t51_malformed_success_is_not_publication() {
    let server = test_support::server_raw(vec![(202, "{}".into()), (202, "invalid-json".into())]);
    assert!(matches!(
        server.client().aggregation_publish(AppId::Claude, &publish()).await,
        Err(AggregationApiError::Decode(_))
    ));
    assert!(matches!(
        server.client().aggregation_exit(AppId::Codex, &exit()).await,
        Err(AggregationApiError::Decode(_))
    ));
    server.join();
}
