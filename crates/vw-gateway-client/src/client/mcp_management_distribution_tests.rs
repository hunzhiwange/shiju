use super::mcp_management_distribution::*;
use super::{AgentAppsSettingsError, GatewayClient, test_support};
use crate::endpoint::GatewayAuth;
use serde_json::{Value, json};
use vw_api_types::{
    TenantScope,
    agent_apps::{AppId, AppInstallationId},
    mcp_management::*,
    task::TaskStatus,
};

fn target() -> McpAppTarget {
    McpAppTarget {
        app_id: AppId::Codex,
        installation_id: AppInstallationId("install /中文".into()),
        target_host_id: "remote /?中文".into(),
    }
}
fn binding(enabled: bool) -> McpBindingRequest {
    McpBindingRequest {
        target: target(),
        project_id: Some("project".into()),
        expected_revision: 8,
        enabled,
    }
}
fn result() -> Value {
    json!({"outcome":"partially_succeeded","items":[
        {"record":{"id":"docs","revision":9},"target":target(),"enabled":true,"desiredCommitted":true,"nativeOutcome":{"status":"applied"}},
        {"record":{"id":"other","revision":3},"target":target(),"enabled":true,"desiredCommitted":true,"nativeOutcome":{"status":"failed","code":"native_write_failed"}}
    ]})
}

#[tokio::test]
async fn cc_mcp_t42_binding_uses_put_and_preserves_scoped_partial_results() {
    let server = test_support::server_with_tenant_scope(
        vec![(200, result())],
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
    let body = binding(true);
    let actual = client
        .mcp_management_bind(&McpRecordId("文档 /?&+".into()), &body, "fixture-grant")
        .await
        .unwrap();
    assert_eq!(actual, serde_json::from_value::<McpDistributionResult>(result()).unwrap());
    assert_eq!(actual.outcome, McpBatchOutcome::PartiallySucceeded);
    assert!(actual.items.iter().all(|item| item.desired_committed));
    let request = server.take_request();
    assert_eq!(request.method, "PUT");
    assert_eq!(request.path, "/v1/agent-apps/mcp/%E6%96%87%E6%A1%A3%20%2F%3F&+/bindings");
    assert_eq!(request.body, serde_json::to_value(body).unwrap());
    for (name, value) in [
        ("authorization", "Bearer fixture-skey"),
        ("x-agent-apps-token", "fixture-grant"),
        ("x-platform-id", "p"),
        ("x-company-id", "c"),
    ] {
        assert_eq!(request.header(name), Some(value));
    }
    server.join();
}

#[tokio::test]
async fn cc_mcp_t42_import_preserves_source_and_entry_failures() {
    let response = json!({"sources":[
        {"target":target(),"entries":[
            {"id":"a","status":"added","record":{"id":"a","revision":1}},
            {"id":"b","status":"merged","record":{"id":"b","revision":2}},
            {"id":"c","status":"unchanged","record":{"id":"c","revision":3}},
            {"id":null,"status":"failed","code":"invalid_definition"}]},
        {"target":{"appId":"claude","installationId":"official","targetHostId":"remote"},"code":"credential_unavailable"}
    ]});
    let body = McpImportRequest {
        project_id: Some("project".into()),
        sources: vec![
            McpImportSource { target: target(), account_id: None },
            McpImportSource {
                target: McpAppTarget {
                    app_id: AppId::Claude,
                    installation_id: AppInstallationId("official".into()),
                    target_host_id: "remote".into(),
                },
                account_id: Some("account-ref".into()),
            },
        ],
    };
    let server = test_support::server(vec![(200, response.clone())]);
    let actual = server.client().mcp_management_import(&body, "fixture-grant").await.unwrap();
    assert_eq!(actual, serde_json::from_value::<McpImportResponse>(response).unwrap());
    assert_eq!(actual.sources[1].code, Some(McpFailureCode::CredentialUnavailable));
    assert!(matches!(
        actual.sources[0].entries.as_ref().unwrap()[3].outcome,
        McpImportEntryOutcome::Failed { code: McpFailureCode::InvalidDefinition }
    ));
    let request = server.take_request();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/agent-apps/mcp/import");
    assert_eq!(request.header("x-agent-apps-token"), Some("fixture-grant"));
    assert_eq!(request.body, serde_json::to_value(body).unwrap());
    server.join();
}

#[tokio::test]
async fn cc_mcp_t42_bulk_sends_enabled_and_disabled_revisions() {
    let server = test_support::server(vec![
        (200, result()),
        (
            200,
            json!({"outcome":"failed","items":[{"record":{"id":"docs","revision":8},"target":target(),"enabled":false,"desiredCommitted":false,"nativeOutcome":{"status":"failed","code":"stale_revision"}}]}),
        ),
    ]);
    for enabled in [true, false] {
        let body = McpBulkRequest {
            target: target(),
            project_id: None,
            records: vec![
                McpRecordRef { id: McpRecordId("docs".into()), revision: 8 },
                McpRecordRef { id: McpRecordId("other".into()), revision: 2 },
            ],
            enabled,
        };
        let actual = server.client().mcp_management_bulk(&body, "fixture-grant").await.unwrap();
        assert_eq!(
            actual.outcome,
            if enabled { McpBatchOutcome::PartiallySucceeded } else { McpBatchOutcome::Failed }
        );
        if !enabled {
            assert!(!actual.items[0].desired_committed);
        }
        let request = server.take_request();
        assert_eq!(request.method, "POST");
        assert_eq!(request.path, "/v1/agent-apps/mcp/bulk");
        assert_eq!(request.header("x-agent-apps-token"), Some("fixture-grant"));
        assert_eq!(request.body, serde_json::to_value(body).unwrap());
    }
    server.join();
}

#[tokio::test]
async fn cc_mcp_t42_resync_returns_existing_task_reference_for_owned_poll_and_cancel() {
    let server = test_support::server(vec![
        (202, json!({"operationId":"job /?","status":"queued"})),
        (
            200,
            json!({"operationId":"job /?","status":"running","phase":"executing","targets":[],"outcome":null}),
        ),
        (
            200,
            json!({"operationId":"job /?","status":"cancelled","phase":"terminal","targets":[],"outcome":"cancelled"}),
        ),
    ]);
    let body = McpResyncRequest {
        target_host_id: "remote /?中文".into(),
        project_id: Some("project".into()),
        targets: vec![McpResyncTarget {
            app_id: AppId::Claude,
            installation_id: AppInstallationId("official".into()),
            account_id: Some("account-ref".into()),
        }],
    };
    let client = server.client();
    let accepted = client.mcp_management_resync(&body, "fixture-grant").await.unwrap();
    assert_eq!(accepted.status, TaskStatus::Queued);
    assert_eq!(accepted.operation_id.0, "job /?");
    assert_eq!(
        client
            .agent_apps_operation_authorized(
                &body.target_host_id,
                &accepted.operation_id,
                "fixture-grant"
            )
            .await
            .unwrap()
            .status,
        TaskStatus::Running
    );
    assert_eq!(
        client
            .agent_apps_operation_cancel_authorized(
                &body.target_host_id,
                &accepted.operation_id,
                "fixture-grant"
            )
            .await
            .unwrap()
            .status,
        TaskStatus::Cancelled
    );
    for method in ["POST", "GET", "DELETE"] {
        let request = server.take_request();
        assert_eq!(request.method, method);
        assert_eq!(request.header("x-agent-apps-token"), Some("fixture-grant"));
        if method == "POST" {
            assert_eq!(request.path, "/v1/agent-apps/mcp/resync");
            assert_eq!(request.body, serde_json::to_value(&body).unwrap());
        } else {
            let url = reqwest::Url::parse(&format!("http://fixture{}", request.path)).unwrap();
            assert_eq!(url.path(), "/v1/agent-apps/operations/job%20%2F%3F");
            assert_eq!(
                url.query_pairs().collect::<Vec<_>>(),
                vec![("targetHostId".into(), body.target_host_id.as_str().into())]
            );
            assert!(request.body.is_null());
        }
    }
    server.join();
}

#[tokio::test]
async fn cc_mcp_t42_preserves_http_decode_and_transport_errors() {
    let id = McpRecordId("docs".into());
    for status in [400, 401, 403, 404, 409, 422, 503, 307] {
        let body = json!({"code":"permission_denied"});
        let server = test_support::server(vec![(status, body.clone())]);
        assert_eq!(
            server
                .client()
                .mcp_management_bind(&id, &binding(true), "fixture-grant")
                .await
                .unwrap_err(),
            AgentAppsSettingsError::Http { status, body }
        );
        server.join();
    }
    let server = test_support::server_raw(vec![(503, "unavailable".into()), (200, "{}".into())]);
    assert_eq!(
        server
            .client()
            .mcp_management_bind(&id, &binding(true), "fixture-grant")
            .await
            .unwrap_err(),
        AgentAppsSettingsError::Http { status: 503, body: json!("unavailable") }
    );
    assert!(matches!(
        server.client().mcp_management_bind(&id, &binding(true), "fixture-grant").await,
        Err(AgentAppsSettingsError::Decode(_))
    ));
    server.join();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let client =
        GatewayClient::new(crate::endpoint::GatewayEndpoint::new("127.0.0.1", port)).unwrap();
    for id in ["", ".", ".."] {
        assert_eq!(
            client
                .mcp_management_bind(&McpRecordId(id.into()), &binding(true), "fixture-grant")
                .await
                .unwrap_err(),
            AgentAppsSettingsError::Transport("invalid MCP record identity".into())
        );
    }
    assert_eq!(
        client.mcp_management_bind(&id, &binding(true), "bad\ncredential").await.unwrap_err(),
        AgentAppsSettingsError::Transport("invalid application credential".into())
    );
    assert!(matches!(
        client.mcp_management_bind(&id, &binding(true), "fixture-grant").await,
        Err(AgentAppsSettingsError::Transport(_))
    ));
}

#[tokio::test]
async fn cc_mcp_t42_redirect_does_not_forward_application_credentials() {
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
        client
            .mcp_management_bind(&McpRecordId("docs".into()), &binding(true), "fixture-grant")
            .await
            .unwrap_err(),
        AgentAppsSettingsError::Http { status: 307, body: json!({}) }
    );
    worker.join().unwrap();
    assert_eq!(destination.accept().unwrap_err().kind(), std::io::ErrorKind::WouldBlock);
}

#[tokio::test]
async fn cc_mcp_t42_resync_status_retains_target_reports_and_batch_outcome() {
    let terminal = json!({"operationId":"job /?","status":"failed","phase":"terminal",
    "outcome":"partially_succeeded","targets":[
        {"appId":"codex","distribution":result()},
        {"appId":"claude","code":"credential_unavailable"}
    ]});
    let running = json!({"operationId":"job /?","status":"running","phase":"executing",
        "outcome":null,"targets":[{"appId":"codex","distribution":result()}]});
    let cancelled = json!({"operationId":"job /?","status":"cancelled","phase":"terminal",
    "outcome":"cancelled","targets":[
        {"appId":"codex","distribution":result()},
        {"appId":"gemini","distribution":{"outcome":"cancelled","items":[]}}
    ]});
    let server = test_support::server_with_tenant_scope(
        vec![
            (200, running.clone()),
            (200, terminal.clone()),
            (200, cancelled.clone()),
            (200, terminal.clone()),
            (200, terminal.clone()),
        ],
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
    let host = "remote /?中文";
    let id = "job /?".into();
    let actual = client.mcp_management_resync_status(host, &id, "fixture-grant").await.unwrap();
    assert_eq!(actual.operation.status, TaskStatus::Running);
    assert_eq!(actual.outcome, None);
    assert_eq!(actual.targets.len(), 1);
    assert_eq!(serde_json::to_value(actual).unwrap(), running);
    let actual = client.mcp_management_resync_status(host, &id, "fixture-grant").await.unwrap();
    assert_eq!(actual.operation.status, TaskStatus::Failed);
    assert_eq!(actual.outcome, Some(McpBatchOutcome::PartiallySucceeded));
    assert_eq!(
        actual.targets[0].distribution,
        Some(serde_json::from_value::<McpDistributionResult>(result()).unwrap())
    );
    assert_eq!(actual.targets[1].code, Some(McpFailureCode::CredentialUnavailable));
    assert_eq!(serde_json::to_value(actual).unwrap(), terminal);
    let actual = client.mcp_management_resync_cancel(host, &id, "fixture-grant").await.unwrap();
    assert_eq!(actual.operation.status, TaskStatus::Cancelled);
    assert_eq!(actual.outcome, Some(McpBatchOutcome::Cancelled));
    assert_eq!(serde_json::to_value(actual).unwrap(), cancelled);
    let completed_cancel =
        client.mcp_management_resync_cancel(host, &id, "fixture-grant").await.unwrap();
    assert_eq!(serde_json::to_value(completed_cancel).unwrap(), terminal);
    // Existing generic callers retain their original state-only contract.
    let generic = client.agent_apps_operation_authorized(host, &id, "fixture-grant").await.unwrap();
    assert_eq!(
        serde_json::to_value(generic).unwrap(),
        json!({"operationId":"job /?","status":"failed","phase":"terminal"})
    );
    for method in ["GET", "GET", "DELETE", "DELETE", "GET"] {
        let request = server.take_request();
        assert_eq!(request.method, method);
        assert!(request.body.is_null());
        let url = reqwest::Url::parse(&format!("http://fixture{}", request.path)).unwrap();
        assert_eq!(url.path(), "/v1/agent-apps/operations/job%20%2F%3F");
        assert_eq!(
            url.query_pairs().collect::<Vec<_>>(),
            vec![("targetHostId".into(), host.into())]
        );
        for (name, value) in [
            ("authorization", "Bearer fixture-skey"),
            ("x-agent-apps-token", "fixture-grant"),
            ("x-platform-id", "p"),
            ("x-company-id", "c"),
        ] {
            assert_eq!(request.header(name), Some(value));
        }
    }
    server.join();
}

#[tokio::test]
async fn cc_mcp_t42_resync_status_preserves_errors_and_rejects_missing_reports() {
    use super::AgentAppsLifecycleError;
    let id = "job".into();
    for status in [401, 403, 404, 503, 307] {
        let body = json!({"code":"permission_denied"});
        let server = test_support::server(vec![(status, body.clone())]);
        assert_eq!(
            server
                .client()
                .mcp_management_resync_status("remote", &id, "fixture-grant")
                .await
                .unwrap_err(),
            AgentAppsLifecycleError::Http { status, body }
        );
        server.join();
    }
    let server = test_support::server_raw(vec![
        (503, "unavailable".into()),
        (200, json!({"operationId":"job","status":"running","phase":"executing"}).to_string()),
        (200, json!({"operationId":"job","status":"failed","phase":"terminal","targets":[],"outcome":"unknown"}).to_string()),
        (200, "not JSON".into()),
    ]);
    assert_eq!(
        server
            .client()
            .mcp_management_resync_cancel("remote", &id, "fixture-grant")
            .await
            .unwrap_err(),
        AgentAppsLifecycleError::Http { status: 503, body: json!("unavailable") }
    );
    for _ in 0..3 {
        assert!(matches!(
            server.client().mcp_management_resync_status("remote", &id, "fixture-grant").await,
            Err(AgentAppsLifecycleError::Decode(_))
        ));
    }
    server.join();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let client =
        GatewayClient::new(crate::endpoint::GatewayEndpoint::new("127.0.0.1", port)).unwrap();
    for value in ["", ".", ".."] {
        assert_eq!(
            client
                .mcp_management_resync_status("remote", &value.into(), "fixture-grant")
                .await
                .unwrap_err(),
            AgentAppsLifecycleError::Transport("invalid MCP resync operation identity".into())
        );
    }
    assert_eq!(
        client.mcp_management_resync_status("remote", &id, "bad\ncredential").await.unwrap_err(),
        AgentAppsLifecycleError::Transport("invalid application credential".into())
    );
    assert!(matches!(
        client.mcp_management_resync_cancel("remote", &id, "fixture-grant").await,
        Err(AgentAppsLifecycleError::Transport(_))
    ));
}
