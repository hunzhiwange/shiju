use super::{GatewayClient, test_support};
use serde_json::json;
use vw_api_types::agent_apps::aggregation::*;
fn scope() -> AggregationQuery {
    AggregationQuery { target_host_id: "host & one".into(), project_id: "project/one".into() }
}
fn settings() -> serde_json::Value {
    json!({"revision":"settings-r1","requested":{"subagentModelId":"saved-model","subagentEffort":"high","autoReviewModelId":null,"classicTools":true},"effective":{"subagentModelId":null,"subagentEffort":null,"autoReviewModelId":null,"classicTools":false},"choices":[],"notices":[]})
}
#[tokio::test]
async fn aggregation_t52_aggregation_codex_api_preserves_contract() {
    let server = test_support::server(vec![(200, settings())]);
    let result = server.client().aggregation_codex_settings(&scope()).await;
    assert!(result.is_ok(), "settings must reach the scoped gateway: {result:?}");
    assert_eq!(serde_json::to_value(result.unwrap()).unwrap(), settings());
    let request = server.take_request();
    assert_eq!(request.method, "GET");
    let url = reqwest::Url::parse(&format!("{}{}", server.base_url(), request.path)).unwrap();
    assert_eq!(url.path(), "/v1/agent-apps/codex/aggregation/codex/settings");
    let query = url.query_pairs().collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(query["targetHostId"], scope().target_host_id);
    assert_eq!(query["projectId"], scope().project_id.as_ref());
    server.join();
}
fn clients() -> serde_json::Value {
    json!({"revision":"clients-r2","daemon":true,"others":true,"auth":true,"unverified":true})
}
fn restart(confirmed: bool) -> CodexRestartRequest {
    CodexRestartRequest {
        target_host_id: scope().target_host_id,
        project_id: scope().project_id,
        expected_revision: "clients-r2".into(),
        request_id: "request-one".into(),
        confirmed,
    }
}
fn save() -> CodexSettingsRequest {
    CodexSettingsRequest {
        target_host_id: scope().target_host_id,
        project_id: scope().project_id,
        expected_revision: "settings-r1".into(),
        requested: serde_json::from_value(settings()["requested"].clone()).unwrap(),
    }
}
fn ack() -> CodexClientsAckRequest {
    CodexClientsAckRequest {
        target_host_id: scope().target_host_id,
        project_id: scope().project_id,
        expected_revision: "clients-r2".into(),
    }
}
fn progress() -> serde_json::Value {
    json!({"operationId":"operation-one","requestId":"request-one","status":"running","phase":"settling","completed":0,"total":1,"cancellationRequested":false})
}
fn client(server: &test_support::TestServer) -> GatewayClient {
    GatewayClient::new(
        server
            .client()
            .endpoint
            .clone()
            .with_auth(crate::GatewayAuth { skey: Some("fixture-bearer".into()) }),
    )
    .unwrap()
}
#[tokio::test]
async fn aggregation_t52_write_ack_restart_preserve_wire_contract_and_identity() {
    let server = test_support::server_with_tenant_scope(
        vec![(200, settings()), (200, clients()), (200, clients()), (202, progress())],
        vw_api_types::id::TenantScope::new("platform", "company").unwrap(),
    );
    let client = client(&server);
    assert_eq!(
        serde_json::to_value(client.aggregation_codex_settings_save(&save()).await.unwrap())
            .unwrap(),
        settings()
    );
    assert_eq!(
        serde_json::to_value(client.aggregation_codex_clients(&scope()).await.unwrap()).unwrap(),
        clients()
    );
    assert_eq!(
        serde_json::to_value(client.aggregation_codex_ack(&ack()).await.unwrap()).unwrap(),
        clients()
    );
    assert_eq!(
        serde_json::to_value(client.aggregation_codex_restart(&restart(true)).await.unwrap())
            .unwrap(),
        progress()
    );
    for (method, resource, body) in [
        ("PUT", "settings", serde_json::to_value(save()).unwrap()),
        ("GET", "clients", serde_json::Value::Null),
        ("POST", "ack", serde_json::to_value(ack()).unwrap()),
        ("POST", "restart", serde_json::to_value(restart(true)).unwrap()),
    ] {
        let request = server.take_request();
        assert_eq!(request.method, method);
        let url = reqwest::Url::parse(&format!("{}{}", server.base_url(), request.path)).unwrap();
        assert_eq!(url.path(), format!("/v1/agent-apps/codex/aggregation/codex/{resource}"));
        if method == "GET" {
            let query = url.query_pairs().collect::<std::collections::BTreeMap<_, _>>();
            assert_eq!(query["targetHostId"], scope().target_host_id);
            assert_eq!(query["projectId"], scope().project_id.as_ref());
        }
        assert_eq!(request.body, body);
        assert_eq!(request.header("authorization"), Some("Bearer fixture-bearer"));
        assert_eq!(request.header("x-platform-id"), Some("platform"));
        assert_eq!(request.header("x-company-id"), Some("company"));
        assert!(request.header("x-agent-apps-token").is_none());
    }
    server.join();
}
#[tokio::test]
async fn aggregation_t52_errors_and_explicit_rejected_confirmation_are_preserved() {
    for (status, code) in [
        (401, "unauthenticated"),
        (403, "forbidden"),
        (409, "revision_conflict"),
        (409, "request_conflict"),
        (409, "recovery_pending"),
        (422, "confirmation_required"),
        (422, "unsupported_target"),
        (502, "target_failed"),
        (503, "target_unavailable"),
        (504, "target_timeout"),
    ] {
        let body = json!({"error":{"code":code,"message":code,"details":null}});
        let server = test_support::server(vec![(status, body.clone()); 5]);
        let client = server.client();
        let errors = [
            client.aggregation_codex_settings(&scope()).await.unwrap_err(),
            client.aggregation_codex_settings_save(&save()).await.unwrap_err(),
            client.aggregation_codex_clients(&scope()).await.unwrap_err(),
            client.aggregation_codex_ack(&ack()).await.unwrap_err(),
            client.aggregation_codex_restart(&restart(false)).await.unwrap_err(),
        ];
        for error in errors {
            assert_eq!(error, super::AgentAppsSettingsError::Http { status, body: body.clone() });
        }
        for _ in 0..4 {
            server.take_request();
        }
        let request = server.take_request();
        assert_eq!(request.body["confirmed"], false);
        assert_eq!(request.body, serde_json::to_value(restart(false)).unwrap());
        server.join();
    }
    let mut body = serde_json::to_value(restart(true)).unwrap();
    body.as_object_mut().unwrap().remove("confirmed");
    assert!(serde_json::from_value::<CodexRestartRequest>(body).is_err());
    for key in ["command", "pid", "path", "args", "env"] {
        let mut body = serde_json::to_value(restart(true)).unwrap();
        body[key] = json!("untrusted");
        assert!(serde_json::from_value::<CodexRestartRequest>(body).is_err());
    }
}
#[tokio::test]
async fn aggregation_t52_restart_poll_cancel_retain_real_terminal_meaning() {
    for result in ["restarted", "not_running"] {
        let mut terminal = progress();
        terminal["status"] = json!("completed");
        terminal["phase"] = json!("terminal");
        terminal["aggregationOutcome"] = json!({"kind":"restarted","result":result});
        let mut settling = progress();
        settling["cancellationRequested"] = json!(true);
        let server = test_support::server(vec![
            (202, progress()),
            (200, settling.clone()),
            (200, terminal.clone()),
        ]);
        let client = server.client();
        client.aggregation_codex_restart(&restart(true)).await.unwrap();
        let cancelled =
            client.provider_aggregation_operation_cancel("operation-one", &scope()).await.unwrap();
        assert_eq!(serde_json::to_value(cancelled).unwrap(), settling);
        let done =
            client.provider_aggregation_operation_get("operation-one", &scope()).await.unwrap();
        assert_eq!(serde_json::to_value(done).unwrap(), terminal);
        assert_eq!(server.take_request().method, "POST");
        for method in ["DELETE", "GET"] {
            let request = server.take_request();
            assert_eq!(request.method, method);
            let url =
                reqwest::Url::parse(&format!("{}{}", server.base_url(), request.path)).unwrap();
            assert_eq!(url.path(), "/v1/app-providers/operations/operation-one");
            let query = url.query_pairs().collect::<std::collections::BTreeMap<_, _>>();
            assert_eq!(query["targetHostId"], scope().target_host_id);
            assert_eq!(query["projectId"], scope().project_id.as_ref());
        }
        server.join();
    }
}
#[tokio::test]
async fn aggregation_t52_malformed_and_unexpected_status_never_succeed() {
    for body in ["null", "{}", "not JSON"] {
        let server = test_support::server_raw(vec![(200, body.into()), (202, body.into())]);
        assert!(matches!(
            server.client().aggregation_codex_settings(&scope()).await,
            Err(super::AgentAppsSettingsError::Decode(_))
        ));
        assert!(matches!(
            server.client().aggregation_codex_restart(&restart(true)).await,
            Err(super::AgentAppsSettingsError::Decode(_))
        ));
        server.take_request();
        server.take_request();
        server.join();
    }
    for (status, body) in [(202, settings()), (307, settings())] {
        let server = test_support::server(vec![(status, body)]);
        assert!(
            matches!(server.client().aggregation_codex_settings(&scope()).await,Err(super::AgentAppsSettingsError::Http{status:s,..}) if s==status)
        );
        server.take_request();
        server.join();
    }
    let server = test_support::server(vec![(200, progress())]);
    assert!(matches!(
        server.client().aggregation_codex_restart(&restart(true)).await,
        Err(super::AgentAppsSettingsError::Http { status: 200, .. })
    ));
    server.take_request();
    server.join();
}

#[tokio::test]
async fn aggregation_t52_ambiguous_writes_are_not_replayed() {
    use std::{
        io::Read,
        net::TcpListener,
        sync::{
            Arc,
            atomic::{AtomicBool, AtomicUsize, Ordering},
        },
        time::Duration,
    };
    for action in ["settings", "ack", "restart"] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        listener.set_nonblocking(true).unwrap();
        let stopped = Arc::new(AtomicBool::new(false));
        let count = Arc::new(AtomicUsize::new(0));
        let thread_stop = stopped.clone();
        let thread_count = count.clone();
        let worker =
            std::thread::spawn(move || {
                while !thread_stop.load(Ordering::SeqCst) {
                    match listener.accept() {
                        Ok((mut stream, _)) => {
                            stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
                            let mut bytes = [0; 4096];
                            let read = stream.read(&mut bytes).unwrap();
                            assert!(std::str::from_utf8(&bytes[..read]).unwrap().contains(
                                &format!("/v1/agent-apps/codex/aggregation/codex/{action}")
                            ));
                            thread_count.fetch_add(1, Ordering::SeqCst);
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(2))
                        }
                        Err(error) => panic!("fixture accept: {error}"),
                    }
                }
            });
        let client = GatewayClient::new(crate::GatewayEndpoint::new("127.0.0.1", port)).unwrap();
        let result = tokio::time::timeout(Duration::from_secs(5), async {
            match action {
                "settings" => client.aggregation_codex_settings_save(&save()).await.map(|_| ()),
                "ack" => client.aggregation_codex_ack(&ack()).await.map(|_| ()),
                "restart" => client.aggregation_codex_restart(&restart(true)).await.map(|_| ()),
                _ => unreachable!(),
            }
        })
        .await;
        stopped.store(true, Ordering::SeqCst);
        worker.join().unwrap();
        assert!(matches!(result.unwrap(), Err(super::AgentAppsSettingsError::Transport(_))));
        assert_eq!(count.load(Ordering::SeqCst), 1, "ambiguous writes must not be replayed");
    }
}

#[tokio::test]
async fn aggregation_t52_operation_failure_is_not_restart_success() {
    let mut failure = progress();
    failure["status"] = json!("failed");
    failure["phase"] = json!("terminal");
    failure["aggregationOutcome"] =
        json!({"kind":"failed","status":502,"code":"target_failed","recovery":"none"});
    let server = test_support::server(vec![(200, failure.clone())]);
    let result = server
        .client()
        .provider_aggregation_operation_get("operation-one", &scope())
        .await
        .unwrap();
    assert!(matches!(
        result.aggregation_outcome,
        Some(AggregationOutcome::Failed {
            status: 502,
            code: AggregationErrorCode::TargetFailed,
            ..
        })
    ));
    assert_eq!(serde_json::to_value(result).unwrap(), failure);
    server.take_request();
    server.join();
}
