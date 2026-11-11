use super::test_support;
use crate::{AggregationApiError, GatewayClient};
use serde_json::{Value, json};
use vw_api_types::agent_apps::{AppId, aggregation::*};

fn state() -> Value {
    json!({"revision":"opaque-revision","mode":"aggregation","attached":true,"defaultProviderId":"server-default","directProviderId":"direct","members":[{"providerId":"server-member","name":"成员","models":[{"id":"opaque-model","label":"模型","supports1m":true}]}],"notices":["new_session"],"recovery":"pending"})
}
fn scope() -> AggregationQuery {
    AggregationQuery {
        target_host_id: "host /?中文".into(), project_id: "project /?中文".into()
    }
}
fn member() -> AggregationMemberRequest {
    let s = scope();
    AggregationMemberRequest {
        target_host_id: s.target_host_id,
        project_id: s.project_id,
        expected_revision: "opaque /?revision".into(),
        enabled: false,
    }
}
#[tokio::test]
async fn aggregation_t50_aggregation_members_api_preserves_contract() {
    let preview = json!({"revision":"preview-revision","previewToken":null,"defaultProviderId":null,"members":[],"issues":[{"code":"missing_default","blocking":true,"providerId":null,"modelId":null}],"notices":["restart_desktop"]});
    let server = test_support::server(vec![
        (200, state()),
        (200, preview.clone()),
        (200, state()),
        (200, state()),
    ]);
    let endpoint = server
        .client()
        .endpoint()
        .clone()
        .with_auth(crate::GatewayAuth { skey: Some("test-aggregation-grant".into()) });
    let client = GatewayClient::new(endpoint).unwrap();
    assert_eq!(
        serde_json::to_value(client.aggregation_state(AppId::Claude, &scope()).await.unwrap())
            .unwrap(),
        state()
    );
    let s = scope();
    let request = AggregationPreviewRequest {
        target_host_id: s.target_host_id,
        project_id: s.project_id,
        default_provider_id: Some("candidate /?中文".into()),
        catalog_policy: CatalogPolicy::UseManaged,
    };
    assert_eq!(
        serde_json::to_value(client.aggregation_preview(AppId::Codex, &request).await.unwrap())
            .unwrap(),
        preview
    );
    assert_eq!(
        serde_json::to_value(
            client
                .aggregation_member_set(AppId::ClaudeDesktop, "provider /?中文%", &member())
                .await
                .unwrap()
        )
        .unwrap(),
        state()
    );
    let s = scope();
    let default = AggregationDefaultRequest {
        target_host_id: s.target_host_id,
        project_id: s.project_id,
        expected_revision: "revision-default".into(),
        provider_id: "new /?中文".into(),
    };
    assert_eq!(
        serde_json::to_value(
            client.aggregation_default_set(AppId::Claude, &default).await.unwrap()
        )
        .unwrap(),
        state()
    );
    for (method, path, body) in [
        (
            "GET",
            "/v1/agent-apps/claude/aggregation/state?targetHostId=host+%2F%3F%E4%B8%AD%E6%96%87&projectId=project+%2F%3F%E4%B8%AD%E6%96%87",
            Value::Null,
        ),
        (
            "POST",
            "/v1/agent-apps/codex/aggregation/preview",
            serde_json::to_value(request).unwrap(),
        ),
        (
            "PUT",
            "/v1/agent-apps/claude-desktop/aggregation/members/provider%20%2F%3F%E4%B8%AD%E6%96%87%25",
            serde_json::to_value(member()).unwrap(),
        ),
        (
            "PUT",
            "/v1/agent-apps/claude/aggregation/default",
            serde_json::to_value(default).unwrap(),
        ),
    ] {
        let recorded = server.take_request();
        assert_eq!(recorded.method, method);
        assert_eq!(recorded.path, path);
        assert_eq!(recorded.body, body);
        assert_eq!(recorded.header("authorization"), Some("Bearer test-aggregation-grant"));
        assert_eq!(
            recorded.header("x-platform-id"),
            Some(client.endpoint().tenant_scope().platform_id().as_ref())
        );
        assert_eq!(
            recorded.header("x-company-id"),
            Some(client.endpoint().tenant_scope().company_id().as_ref())
        );
    }
    server.join();
}
#[tokio::test]
async fn aggregation_t50_errors_remain_structured() {
    for (status, code) in [
        (401, "unauthenticated"),
        (403, "forbidden"),
        (409, "revision_conflict"),
        (422, "unsupported_app"),
        (422, "unsupported_target"),
        (502, "target_failed"),
        (503, "target_unavailable"),
        (504, "target_timeout"),
    ] {
        let body =
            json!({"error":{"code":code,"message":"safe error","details":{"recovery":"pending"}}});
        let server = test_support::server(vec![(status, body.clone()); 4]);
        let client = server.client();
        let s = scope();
        let preview = AggregationPreviewRequest {
            target_host_id: s.target_host_id.clone(),
            project_id: s.project_id.clone(),
            default_provider_id: None,
            catalog_policy: CatalogPolicy::Preserve,
        };
        let default = AggregationDefaultRequest {
            target_host_id: s.target_host_id,
            project_id: s.project_id,
            expected_revision: "stale".into(),
            provider_id: "p".into(),
        };
        let expected = AggregationApiError::Http { status, body };
        assert_eq!(client.aggregation_state(AppId::Claude, &scope()).await.unwrap_err(), expected);
        assert_eq!(client.aggregation_preview(AppId::Codex, &preview).await.unwrap_err(), expected);
        assert_eq!(
            client.aggregation_member_set(AppId::Claude, "p", &member()).await.unwrap_err(),
            expected
        );
        assert_eq!(
            client.aggregation_default_set(AppId::Claude, &default).await.unwrap_err(),
            expected
        );
        server.join();
    }
}
#[tokio::test]
async fn aggregation_t50_malformed_response_and_empty_errors_are_explicit() {
    let server = test_support::server_raw(vec![(200, "{}".into()), (502, String::new())]);
    assert!(matches!(
        server.client().aggregation_state(AppId::Claude, &scope()).await,
        Err(AggregationApiError::Decode(_))
    ));
    assert_eq!(
        server.client().aggregation_member_set(AppId::Claude, "p", &member()).await.unwrap_err(),
        AggregationApiError::Http { status: 502, body: json!("") }
    );
    server.join();
}

#[tokio::test]
async fn aggregation_t50_ambiguous_writes_and_redirects_never_replay() {
    use std::{
        io::{BufRead, Read, Write},
        net::TcpListener,
        time::{Duration, Instant},
    };
    for redirect in [false, true] {
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
                            serde_json::to_value(member()).unwrap()
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
        let result = client.aggregation_member_set(AppId::Claude, "p", &member()).await;
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
