use super::{AgentAppsSettingsError, GatewayClient, test_support};
use crate::endpoint::GatewayAuth;
use serde_json::json;
use vw_api_types::{agent_apps::aggregation::*, id::TenantScope};

fn scope() -> AggregationQuery {
    AggregationQuery { target_host_id: "host +&".into(), project_id: "project +&".into() }
}
fn form() -> serde_json::Value {
    json!({"revision":"desktop-r1","form":{"name":"Fixture","baseUrl":"https://fixture.invalid","official":false,"directEditable":false,"mapping":{"mode":"proxy","routes":{"claude-sonnet-5":{"model":"upstream-one","supports1m":true}}},"mappingEditable":true}})
}
fn client(server: &test_support::TestServer) -> GatewayClient {
    GatewayClient::new(
        server
            .client()
            .endpoint()
            .clone()
            .with_auth(GatewayAuth { skey: Some("fixture-bearer".into()) }),
    )
    .unwrap()
}

#[tokio::test]
async fn aggregation_t53_aggregation_desktop_api_preserves_contract() {
    let server = test_support::server_with_tenant_scope(
        vec![(200, form()), (200, form())],
        TenantScope::new("platform", "company").unwrap(),
    );
    let client = client(&server);
    let view = client.aggregation_desktop_mapping("profile", &scope()).await.unwrap();
    assert_eq!(view.revision, "desktop-r1");
    assert_eq!(view.form.mapping.routes["claude-sonnet-5"].model, "upstream-one");
    assert!(view.form.mapping.routes["claude-sonnet-5"].supports_1m);
    let read = server.take_request();
    assert_eq!(read.method, "POST");
    assert_eq!(read.path, "/v1/app-providers/claude-desktop/profile/form/read");
    assert_eq!(read.body, serde_json::to_value(scope()).unwrap());
    assert_eq!(read.header("authorization"), Some("Bearer fixture-bearer"));
    assert_eq!(read.header("x-platform-id"), Some("platform"));
    assert_eq!(read.header("x-company-id"), Some("company"));
    let change = DesktopMappingChange {
        target_host_id: scope().target_host_id,
        project_id: scope().project_id,
        expected_revision: view.revision,
        mapping: view.form.mapping,
    };
    let saved = client.aggregation_desktop_mapping_save("profile", &change).await.unwrap();
    assert_eq!(saved.revision, "desktop-r1");
    assert_eq!(saved.form.mapping, change.mapping);
    let write = server.take_request();
    assert_eq!(write.method, "PUT");
    assert_eq!(write.path, "/v1/app-providers/claude-desktop/profile/model-mapping");
    assert_eq!(write.body, serde_json::to_value(change).unwrap());
    assert_eq!(write.header("authorization"), Some("Bearer fixture-bearer"));
    server.join();
}

#[tokio::test]
async fn aggregation_t53_status_preserves_modes_installation_and_recovery() {
    for (installation, mode, recovery) in [
        ("installed", "official", "none"),
        ("installed", "direct_gateway", "restored"),
        ("not_installed", "routing", "pending"),
        ("unsupported", "aggregation", "failed"),
    ] {
        let body = json!({"revision":"r1","installation":installation,"configured":true,"mode":mode,"appliedProviderId":"profile","notices":["restart_desktop"],"recovery":recovery});
        let server = test_support::server(vec![(200, body.clone())]);
        let status = client(&server).aggregation_desktop_status(&scope()).await.unwrap();
        assert_eq!(serde_json::to_value(status).unwrap(), body);
        let request = server.take_request();
        assert_eq!(request.method, "GET");
        let url = reqwest::Url::parse(&format!("{}{}", server.base_url(), request.path)).unwrap();
        assert_eq!(url.path(), "/v1/agent-apps/claude-desktop/profile/status");
        let query = url.query_pairs().collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(query["targetHostId"], scope().target_host_id);
        assert_eq!(query["projectId"], scope().project_id.as_ref());
        assert_eq!(request.body, serde_json::Value::Null);
        assert_eq!(request.header("authorization"), Some("Bearer fixture-bearer"));
        server.join();
    }
}

#[tokio::test]
async fn aggregation_t53_mapping_errors_preserve_status_code_and_recovery() {
    for (status, code) in [
        (401, "unauthenticated"),
        (403, "forbidden"),
        (409, "revision_conflict"),
        (422, "unsupported_target"),
        (502, "recovery_failed"),
        (503, "target_unavailable"),
        (504, "target_timeout"),
    ] {
        let body = json!({"error":{"code":code,"message":code,"details":{"recovery":"failed"}}});
        let server = test_support::server(vec![(status, body.clone())]);
        let change: DesktopMappingChange = serde_json::from_value(json!({"targetHostId":"host","projectId":"project","expectedRevision":"r1","mapping":{"mode":"proxy","routes":{}}})).unwrap();
        let error =
            client(&server).aggregation_desktop_mapping_save("profile", &change).await.unwrap_err();
        assert_eq!(error, AgentAppsSettingsError::Http { status, body });
        assert_eq!(server.take_request().method, "PUT");
        server.join();
    }
}

#[tokio::test]
async fn aggregation_t53_mapping_rejects_path_identities_before_transport() {
    let client = GatewayClient::new(crate::GatewayEndpoint::new("127.0.0.1", 1)).unwrap();
    for id in ["", "../escape", "a/b", "a?query", "%2e%2e", "中文"] {
        let error = client.aggregation_desktop_mapping(id, &scope()).await.unwrap_err();
        assert!(
            matches!(error, AgentAppsSettingsError::Transport(ref message) if message == "invalid provider identity")
        );
    }
}

#[tokio::test]
async fn aggregation_t53_malformed_response_and_redirect_never_become_success() {
    let server = test_support::server_raw(vec![(200, "not-json".into()), (307, "redirect".into())]);
    let client = client(&server);
    assert!(matches!(
        client.aggregation_desktop_status(&scope()).await,
        Err(AgentAppsSettingsError::Decode(_))
    ));
    assert!(matches!(
        client.aggregation_desktop_mapping("profile", &scope()).await,
        Err(AgentAppsSettingsError::Http { status: 307, .. })
    ));
    server.take_request();
    server.take_request();
    server.join();
}

#[tokio::test]
async fn aggregation_t53_operation_queries_preserve_project_scope() {
    let progress = json!({"operationId":"operation-one","requestId":"request-one","status":"running","phase":"settling","completed":0,"total":1,"cancellationRequested":false});
    let server = test_support::server(vec![(200, progress)]);
    client(&server).provider_aggregation_operation_get("operation-one", &scope()).await.unwrap();
    let request = server.take_request();
    server.join();
    let url = reqwest::Url::parse(&format!("http://fixture.invalid{}", request.path)).unwrap();
    let query = url.query_pairs().collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(query.get("projectId").map(|v| v.as_ref()), Some(scope().project_id.as_ref()));
}

fn state(attached: bool) -> serde_json::Value {
    json!({"revision":"r2","mode":if attached {"aggregation"} else {"direct"},"attached":attached,"defaultProviderId":"profile","directProviderId":"direct","members":[],"notices":["restart_desktop"],"recovery":if attached {"none"} else {"restored"}})
}
fn progress(status: &str, outcome: serde_json::Value, cancelled: bool) -> serde_json::Value {
    let mut result = json!({"operationId":"operation-one","requestId":"request-one","status":status,"phase":if status=="running" {"settling"} else {"terminal"},"completed":0,"total":1,"cancellationRequested":cancelled});
    if !outcome.is_null() {
        result["aggregationOutcome"] = outcome;
    }
    result
}
fn publish() -> AggregationPublishRequest {
    AggregationPublishRequest {
        target_host_id: scope().target_host_id,
        project_id: scope().project_id,
        expected_revision: "r1".into(),
        request_id: "request-one".into(),
        preview_token: "opaque-preview".into(),
        confirmed: true,
    }
}
fn exit() -> AggregationExitRequest {
    AggregationExitRequest {
        target_host_id: scope().target_host_id,
        project_id: scope().project_id,
        expected_revision: "r2".into(),
        request_id: "exit-one".into(),
        confirmed: true,
    }
}

#[tokio::test]
async fn aggregation_t53_preview_publish_exit_and_existing_operation_transport_preserve_outcomes() {
    let preview = json!({"revision":"r1","previewToken":"opaque-preview","defaultProviderId":"profile","members":[],"issues":[],"notices":["restart_desktop"]});
    let running = progress("running", serde_json::Value::Null, false);
    let published = progress("completed", json!({"kind":"published","state":state(true)}), false);
    // A cancellation request after publication does not imply cancellation succeeded.
    let settling = progress("running", serde_json::Value::Null, true);
    let exited = progress("completed", json!({"kind":"exited","state":state(false)}), true);
    let server = test_support::server_with_tenant_scope(
        vec![
            (200, preview.clone()),
            (202, running.clone()),
            (200, published.clone()),
            (202, running.clone()),
            (200, settling.clone()),
            (200, exited.clone()),
        ],
        TenantScope::new("platform", "company").unwrap(),
    );
    let client = client(&server);
    let intent = AggregationPreviewRequest {
        target_host_id: scope().target_host_id,
        project_id: scope().project_id,
        default_provider_id: Some("profile".into()),
        catalog_policy: CatalogPolicy::Preserve,
    };
    let view = client.aggregation_desktop_preview(&intent).await.unwrap();
    assert_eq!(serde_json::to_value(view).unwrap(), preview);
    let request = server.take_request();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/agent-apps/claude-desktop/aggregation/preview");
    assert_eq!(request.body, serde_json::to_value(intent).unwrap());
    let accepted = client.aggregation_desktop_publish(&publish()).await.unwrap();
    assert_eq!(serde_json::to_value(accepted).unwrap(), running);
    let request = server.take_request();
    assert_eq!(request.path, "/v1/agent-apps/claude-desktop/aggregation/publish");
    assert_eq!(request.method, "POST");
    assert_eq!(request.body, serde_json::to_value(publish()).unwrap());
    assert_eq!(request.header("authorization"), Some("Bearer fixture-bearer"));
    assert_eq!(request.header("x-platform-id"), Some("platform"));
    assert_eq!(request.header("x-company-id"), Some("company"));
    let result =
        client.provider_aggregation_operation_get("operation-one", &scope()).await.unwrap();
    assert_eq!(serde_json::to_value(result).unwrap(), published);
    assert_eq!(server.take_request().method, "GET");
    let accepted = client.aggregation_desktop_exit(&exit()).await.unwrap();
    assert_eq!(serde_json::to_value(accepted).unwrap(), running);
    let request = server.take_request();
    assert_eq!(request.path, "/v1/agent-apps/claude-desktop/aggregation/exit");
    assert_eq!(request.method, "POST");
    assert_eq!(request.body, serde_json::to_value(exit()).unwrap());
    let result =
        client.provider_aggregation_operation_cancel("operation-one", &scope()).await.unwrap();
    assert_eq!(serde_json::to_value(result).unwrap(), settling);
    let request = server.take_request();
    assert_eq!(request.method, "DELETE");
    let url = reqwest::Url::parse(&format!("http://fixture.invalid{}", request.path)).unwrap();
    assert_eq!(url.path(), "/v1/app-providers/operations/operation-one");
    let query = url.query_pairs().collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(query["projectId"], scope().project_id.as_ref());
    assert_eq!(query["targetHostId"], scope().target_host_id);
    assert_eq!(request.header("authorization"), Some("Bearer fixture-bearer"));
    assert_eq!(request.header("x-company-id"), Some("company"));
    assert!(request.header("x-agent-apps-token").is_none());
    let result =
        client.provider_aggregation_operation_get("operation-one", &scope()).await.unwrap();
    assert_eq!(serde_json::to_value(result).unwrap(), exited);
    assert_eq!(server.take_request().method, "GET");
    server.join();
}

#[tokio::test]
async fn aggregation_t53_operation_failure_retains_typed_status_code_and_recovery() {
    for recovery in ["none", "pending", "restored", "failed"] {
        let body = progress(
            "failed",
            json!({"kind":"failed","status":502,"code":"recovery_failed","recovery":recovery}),
            false,
        );
        let server = test_support::server(vec![(200, body.clone()), (200, body.clone())]);
        let client = client(&server);
        let read =
            client.provider_aggregation_operation_get("operation-one", &scope()).await.unwrap();
        assert!(matches!(
            read.aggregation_outcome,
            Some(AggregationOutcome::Failed {
                status: 502,
                code: AggregationErrorCode::RecoveryFailed,
                ..
            })
        ));
        assert_eq!(serde_json::to_value(read).unwrap(), body);
        let cancelled =
            client.provider_aggregation_operation_cancel("operation-one", &scope()).await.unwrap();
        assert_eq!(serde_json::to_value(cancelled).unwrap(), body);
        assert_eq!(server.take_request().method, "GET");
        assert_eq!(server.take_request().method, "DELETE");
        server.join();
    }
}

#[tokio::test]
async fn aggregation_t53_lifecycle_http_errors_remain_distinct_from_operation_outcomes() {
    for (status, code) in [
        (401, "unauthenticated"),
        (403, "forbidden"),
        (409, "revision_conflict"),
        (409, "request_conflict"),
        (409, "recovery_pending"),
        (422, "unsupported_target"),
        (422, "confirmation_required"),
        (502, "recovery_failed"),
        (503, "target_unavailable"),
        (504, "target_timeout"),
    ] {
        let body = json!({"error":{"code":code,"message":code,"details":{"recovery":"pending"}}});
        let server = test_support::server(vec![(status, body.clone()); 4]);
        let client = client(&server);
        let mut publish = publish();
        publish.confirmed = false;
        assert_eq!(
            client.aggregation_desktop_publish(&publish).await.unwrap_err(),
            AgentAppsSettingsError::Http { status, body: body.clone() }
        );
        assert_eq!(server.take_request().body["confirmed"], false);
        assert_eq!(
            client.aggregation_desktop_exit(&exit()).await.unwrap_err(),
            AgentAppsSettingsError::Http { status, body: body.clone() }
        );
        assert_eq!(server.take_request().method, "POST");
        for result in [
            client.provider_aggregation_operation_get("operation-one", &scope()).await,
            client.provider_aggregation_operation_cancel("operation-one", &scope()).await,
        ] {
            assert!(
                matches!(result,Err(crate::AgentAppsReadError::Http {status:s,body:ref b}) if s==status && *b==body)
            );
        }
        assert_eq!(server.take_request().method, "GET");
        assert_eq!(server.take_request().method, "DELETE");
        server.join();
    }
}

#[tokio::test]
async fn aggregation_t53_operation_identity_is_validated_before_network() {
    let client = GatewayClient::new(crate::GatewayEndpoint::new("127.0.0.1", 1)).unwrap();
    for id in ["", "../operation", "a/b", "a?projectId=other", "%2f"] {
        for result in [
            client.provider_aggregation_operation_get(id, &scope()).await,
            client.provider_aggregation_operation_cancel(id, &scope()).await,
        ] {
            assert!(
                matches!(result,Err(crate::AgentAppsReadError::Transport(ref message)) if message=="invalid operation identity")
            );
        }
    }
}

#[tokio::test]
async fn aggregation_t53_blocking_preview_is_returned_without_local_publication() {
    let preview = json!({"revision":"r1","previewToken":null,"defaultProviderId":null,"members":[{"providerId":"profile","name":"Fixture","models":[{"id":"claude-sonnet-5","label":"Fixture Sonnet","supports1m":true}]}],"issues":[{"code":"model_collision","blocking":true,"providerId":"profile","modelId":"claude-sonnet-5"}],"notices":["restart_desktop"]});
    let server = test_support::server(vec![(200, preview.clone())]);
    let intent = AggregationPreviewRequest {
        target_host_id: scope().target_host_id,
        project_id: scope().project_id,
        default_provider_id: None,
        catalog_policy: CatalogPolicy::UseManaged,
    };
    let result = client(&server).aggregation_desktop_preview(&intent).await.unwrap();
    assert!(result.preview_token.is_none());
    assert!(result.issues[0].blocking);
    assert_eq!(serde_json::to_value(result).unwrap(), preview);
    let request = server.take_request();
    assert_eq!(request.body, serde_json::to_value(intent).unwrap());
    assert_eq!(request.path, "/v1/agent-apps/claude-desktop/aggregation/preview");
    server.join();
}

#[tokio::test]
async fn aggregation_t53_ambiguous_writes_are_not_replayed() {
    use std::{
        io::Read,
        net::TcpListener,
        sync::{
            Arc,
            atomic::{AtomicBool, AtomicUsize, Ordering},
        },
        time::Duration,
    };
    for publishing in [true, false] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        listener.set_nonblocking(true).unwrap();
        let stopped = Arc::new(AtomicBool::new(false));
        let count = Arc::new(AtomicUsize::new(0));
        let thread_stop = stopped.clone();
        let thread_count = count.clone();
        let worker = std::thread::spawn(move || {
            while !thread_stop.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
                        let mut bytes = [0; 4096];
                        let read = stream.read(&mut bytes).unwrap();
                        assert!(
                            std::str::from_utf8(&bytes[..read])
                                .unwrap()
                                .starts_with("POST /v1/agent-apps/claude-desktop/aggregation/")
                        );
                        thread_count.fetch_add(1, Ordering::SeqCst);
                        // Disconnect after receiving the write, before replying.
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
            if publishing {
                client.aggregation_desktop_publish(&publish()).await
            } else {
                client.aggregation_desktop_exit(&exit()).await
            }
        })
        .await;
        stopped.store(true, Ordering::SeqCst);
        worker.join().unwrap();
        assert!(matches!(result.unwrap(), Err(AgentAppsSettingsError::Transport(_))));
        assert_eq!(count.load(Ordering::SeqCst), 1, "ambiguous write must not be retried");
    }
}
