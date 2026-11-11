use super::{GatewayClient, test_support};
use crate::GatewayAuth;
use serde_json::{Value, json};
use vw_api_types::{TenantScope, agent_apps_diagnostics::DiagnosticScopeQuery};

fn scope() -> DiagnosticScopeQuery {
    serde_json::from_value(
        json!({"projectId":"项目 /?&+", "targetHostId":"host /?&+", "appId":"codex"}),
    )
    .unwrap()
}
fn identity() -> Value {
    json!({"taskId":"job /?&+", "correlationId":"correlation", "kind":"mcp", "scope":{"project":{"platform_id":"p","company_id":"c","project_id":"项目 /?&+"},"targetHostId":"host /?&+","appId":"codex"}})
}
#[tokio::test]
async fn cc_mcp_t43_explicit_check_preserves_scope_and_credentials() {
    let server = test_support::server_with_tenant_scope(
        vec![(202, identity())],
        TenantScope::new("p", "c").unwrap(),
    );
    let client = GatewayClient::new(
        server
            .client()
            .endpoint()
            .clone()
            .with_auth(GatewayAuth { skey: Some("fixture-skey".into()) }),
    )
    .unwrap();
    let result = client
        .mcp_management_check(
            &scope(),
            &[vw_api_types::mcp_management::McpRecordId("docs".into())],
            9,
            "fixture-grant",
        )
        .await;
    assert!(result.is_ok(), "explicit connection job must be accepted: {result:?}");
    assert_eq!(serde_json::to_value(result.unwrap()).unwrap(), identity());
    let request = server.take_request();
    assert_eq!(request.method, "POST");
    assert_eq!(
        request.path,
        "/v1/agent-apps/mcp/checks?projectId=%E9%A1%B9%E7%9B%AE+%2F%3F%26%2B&targetHostId=host+%2F%3F%26%2B&appId=codex"
    );
    assert_eq!(request.body, json!({"serverIds":["docs"],"generation":9}));
    assert_eq!(request.header("authorization"), Some("Bearer fixture-skey"));
    assert_eq!(request.header("x-agent-apps-token"), Some("fixture-grant"));
    assert_eq!(request.header("x-platform-id"), Some("p"));
    assert_eq!(request.header("x-company-id"), Some("c"));
    server.join();
}

fn snapshot(status: &str) -> Value {
    json!({"job":identity(), "state":{"status":status}, "progress":{"completed":1,"total":2}, "results":[{"kind":"mcp_connection_check","scope":{"kind":"tenant","scope":{"platform_id":"p","company_id":"c"}},"generation":9,"item":{"id":"docs","connectionRevision":7,"outcome":{"status":"connected","toolCount":0}}}]})
}

#[tokio::test]
async fn cc_mcp_t43_poll_and_cancel_preserve_states_revision_and_original_job() {
    use vw_api_types::agent_apps_diagnostics::{DiagnosticJobState, DiagnosticResult};
    let states = ["queued", "running", "cancelling", "cancelled", "succeeded", "interrupted"];
    let server = test_support::server(states.iter().map(|state| (200, snapshot(state))).collect());
    for state in states {
        let cancel = matches!(state, "cancelling" | "cancelled");
        let result = if cancel {
            server
                .client()
                .mcp_management_check_cancel(&scope(), &"job /?&+".into(), "fixture-grant")
                .await
        } else {
            server
                .client()
                .mcp_management_check_status(&scope(), &"job /?&+".into(), "fixture-grant")
                .await
        }
        .unwrap();
        assert_eq!(serde_json::to_value(&result).unwrap(), snapshot(state));
        let DiagnosticResult::McpConnectionCheck { generation, item, .. } =
            &result.results.as_ref().unwrap()[0]
        else {
            panic!("revision-bound result")
        };
        assert_eq!(*generation, 9);
        assert_eq!(item.connection_revision, 7);
        assert_eq!(item.id.0, "docs");
        if state == "cancelling" {
            assert_eq!(result.state, DiagnosticJobState::Cancelling {});
        }
        let request = server.take_request();
        assert_eq!(request.method, if cancel { "POST" } else { "GET" });
        let suffix = if cancel { "/cancel" } else { "" };
        assert_eq!(
            request.path,
            format!(
                "/v1/agent-apps/diagnostics/jobs/job%20%2F%3F&+{suffix}?projectId=%E9%A1%B9%E7%9B%AE+%2F%3F%26%2B&targetHostId=host+%2F%3F%26%2B&appId=codex"
            )
        );
        assert_eq!(request.header("x-agent-apps-token"), Some("fixture-grant"));
        assert!(request.body.is_null());
    }
    server.join();
}

#[tokio::test]
async fn cc_mcp_t43_lifecycle_preserves_http_decode_and_transport_errors() {
    use super::AgentAppsSettingsError;
    for status in [400, 401, 403, 404, 409, 422, 503, 307] {
        let body = json!({"code":"denied"});
        let server = test_support::server(vec![(status, body.clone()); 3]);
        assert_eq!(
            server
                .client()
                .mcp_management_check(&scope(), &[], 9, "fixture-grant")
                .await
                .unwrap_err(),
            AgentAppsSettingsError::Http { status, body: body.clone() }
        );
        assert_eq!(
            server
                .client()
                .mcp_management_check_status(&scope(), &"job".into(), "fixture-grant")
                .await
                .unwrap_err(),
            AgentAppsSettingsError::Http { status, body: body.clone() }
        );
        assert_eq!(
            server
                .client()
                .mcp_management_check_cancel(&scope(), &"job".into(), "fixture-grant")
                .await
                .unwrap_err(),
            AgentAppsSettingsError::Http { status, body }
        );
        server.join();
    }
    let server = test_support::server_raw(vec![
        (503, "unavailable".into()),
        (202, "{}".into()),
        (200, "not JSON".into()),
        (200, "{}".into()),
    ]);
    assert_eq!(
        server.client().mcp_management_check(&scope(), &[], 9, "fixture-grant").await.unwrap_err(),
        AgentAppsSettingsError::Http { status: 503, body: json!("unavailable") }
    );
    assert!(matches!(
        server.client().mcp_management_check(&scope(), &[], 9, "fixture-grant").await,
        Err(AgentAppsSettingsError::Decode(_))
    ));
    assert!(matches!(
        server.client().mcp_management_check_status(&scope(), &"job".into(), "fixture-grant").await,
        Err(AgentAppsSettingsError::Decode(_))
    ));
    assert!(matches!(
        server.client().mcp_management_check_cancel(&scope(), &"job".into(), "fixture-grant").await,
        Err(AgentAppsSettingsError::Decode(_))
    ));
    server.join();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let client = GatewayClient::new(crate::GatewayEndpoint::new("127.0.0.1", port)).unwrap();
    for id in ["", ".", ".."] {
        assert!(matches!(
            client.mcp_management_check_status(&scope(), &id.into(), "fixture-grant").await,
            Err(AgentAppsSettingsError::Transport(_))
        ));
        assert!(matches!(
            client.mcp_management_check_cancel(&scope(), &id.into(), "fixture-grant").await,
            Err(AgentAppsSettingsError::Transport(_))
        ));
    }
    assert!(matches!(
        client.mcp_management_check(&scope(), &[], 9, "bad\ncredential").await,
        Err(AgentAppsSettingsError::Transport(_))
    ));
    assert!(matches!(
        client.mcp_management_check(&scope(), &[], 9, "fixture-grant").await,
        Err(AgentAppsSettingsError::Transport(_))
    ));
}

#[tokio::test]
async fn cc_mcp_t43_redirect_does_not_forward_application_credentials() {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    let destination = TcpListener::bind("127.0.0.1:0").unwrap();
    destination.set_nonblocking(true).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let location = format!("http://{}/capture", destination.local_addr().unwrap());
    let worker = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
        let mut bytes = Vec::new();
        let mut byte = [0];
        while !bytes.ends_with(b"\r\n\r\n") {
            stream.read_exact(&mut byte).unwrap();
            bytes.push(byte[0]);
        }
        // Drain the JSON before closing, to avoid a TCP reset replacing the 307 response.
        let headers = String::from_utf8(bytes).unwrap();
        let length = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().unwrap())
            })
            .unwrap();
        stream.read_exact(&mut vec![0; length]).unwrap();
        write!(stream, "HTTP/1.1 307 Temporary Redirect\r\nLocation: {location}\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{{}}").unwrap();
    });
    let client =
        GatewayClient::new(crate::endpoint::GatewayEndpoint::new("127.0.0.1", port)).unwrap();
    assert_eq!(
        client.mcp_management_check(&scope(), &[], 9, "fixture-grant").await.unwrap_err(),
        super::AgentAppsSettingsError::Http { status: 307, body: json!({}) }
    );
    worker.join().unwrap();
    assert_eq!(destination.accept().unwrap_err().kind(), std::io::ErrorKind::WouldBlock);
}

#[tokio::test]
async fn cc_mcp_t43_failed_job_and_per_service_failure_remain_distinct() {
    use vw_api_types::agent_apps_diagnostics::{DiagnosticJobState, DiagnosticResult};
    use vw_api_types::mcp_management::{McpConnectionOutcome, McpFailureCode};
    let mut body = snapshot("failed");
    body["state"]["error"] = json!({"phase":"handshake","code":"timeout"});
    body["results"][0]["item"]["outcome"] = json!({"status":"failed","code":"stale_revision"});
    let server = test_support::server(vec![(200, body.clone())]);
    let result = server
        .client()
        .mcp_management_check_status(&scope(), &"job".into(), "fixture-grant")
        .await
        .unwrap();
    assert!(matches!(result.state, DiagnosticJobState::Failed { .. }));
    let DiagnosticResult::McpConnectionCheck { item, .. } = &result.results.as_ref().unwrap()[0]
    else {
        panic!("connection result")
    };
    assert_eq!(item.outcome, McpConnectionOutcome::Failed { code: McpFailureCode::StaleRevision });
    assert_eq!(serde_json::to_value(result).unwrap(), body);
    server.join();
}
