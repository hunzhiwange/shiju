use super::*;
use crate::app::state::AgentAppsContext;
use vw_gateway_client::{GatewayEndpoint, vw_api_types::id::TenantScope};
fn selection() -> Selection {
    Selection {
        scope: Scope {
            context: AgentAppsContext {
                gateway: "http://127.0.0.1:1".into(),
                tenant: TenantScope::default(),
                project: None,
            },
            host: "host".into(),
            app: AppId::Claude,
        },
        project: "project".into(),
        catalog_generation: 1,
    }
}
#[test]
fn aggregation_t56_aggregation_list_preserves_contract() {
    let old = selection();
    let mut new = old.clone();
    new.scope.app = AppId::Codex;
    let mut form = Form::new(new.clone(), 2);
    assert!(!form.finish(&old, 1, Err(AggregationApiError::Decode("late".into()))));
    assert!(form.loading);
    assert!(form.error.is_none());
    assert!(!form.finish(&new, 1, Err(AggregationApiError::Decode("late".into()))));
    assert!(form.loading);
}
fn state() -> AggregationState {
    serde_json::from_value(serde_json::json!({"revision":"opaque","mode":"aggregation","attached":true,"defaultProviderId":"one","directProviderId":null,"members":[{"providerId":"one","name":"中文默认供应商","models":[{"id":"model-1","label":"模型","supports1m":false}]}],"notices":["new_session"],"recovery":"none"})).unwrap()
}
fn server(
    responses: Vec<(u16, serde_json::Value)>,
) -> (GatewayClient, std::thread::JoinHandle<Vec<String>>) {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = std::thread::spawn(move || {
        let mut requests = Vec::new();
        for (status, body) in responses {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            let mut stream = loop {
                match listener.accept() {
                    Ok((s, _)) => break s,
                    Err(e)
                        if e.kind() == std::io::ErrorKind::WouldBlock
                            && std::time::Instant::now() < deadline =>
                    {
                        std::thread::sleep(std::time::Duration::from_millis(5))
                    }
                    Err(e) => panic!("accept failed: {e}"),
                }
            };
            stream.set_nonblocking(false).unwrap();
            stream.set_read_timeout(Some(std::time::Duration::from_secs(10))).unwrap();
            let mut head = Vec::new();
            while !head.ends_with(b"\r\n\r\n") {
                let mut b = [0];
                stream.read_exact(&mut b).unwrap();
                head.push(b[0]);
            }
            requests.push(String::from_utf8(head).unwrap());
            let body = body.to_string();
            write!(stream, "HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
        }
        requests
    });
    (
        GatewayClient::new(GatewayEndpoint::new("127.0.0.1", port).with_auth(
            vw_gateway_client::GatewayAuth { skey: Some("fixture-gateway-key".into()) },
        ))
        .unwrap(),
        handle,
    )
}
#[tokio::test]
async fn aggregation_t56_sdk_reads_preserve_default_members_and_candidates_without_writes() {
    let catalog = serde_json::json!({"appId":"claude","rows":[{"provider":{"id":"one","appId":"claude","name":"旧名","configured":true},"nativeMember":"absent"},{"provider":{"id":"two","appId":"claude","name":"候选供应商","configured":true},"nativeMember":"present"}],"direct":{"status":"unconfigured"},"inUse":{"status":"unconfigured"},"nativeRead":null});
    for app in [AppId::Claude, AppId::Codex, AppId::ClaudeDesktop] {
        let mut catalog = catalog.clone();
        catalog["appId"] = serde_json::to_value(app).unwrap();
        for row in catalog["rows"].as_array_mut().unwrap() {
            row["provider"]["appId"] = serde_json::to_value(app).unwrap();
        }
        let (client, handle) =
            server(vec![(200, serde_json::to_value(state()).unwrap()), (200, catalog)]);
        let mut selection = selection();
        selection.scope.app = app;
        selection.scope.host = "host /?中文".into();
        selection.project = "project /?中文".into();
        let snapshot =
            load(client, selection.clone(), "fixture-management-grant".into()).await.unwrap();
        assert_eq!(snapshot.state, state());
        assert_eq!(snapshot.candidates.len(), 1);
        assert_eq!(snapshot.candidates[0].id, "two");
        let mut form = Form::new(selection.clone(), 1);
        assert!(form.finish(&selection, 1, Ok(snapshot)));
        assert!(!form.loading);
        assert!(form.error.is_none());
        assert!(!form.finish(&selection, 1, Err(AggregationApiError::Decode("duplicate".into()))));
        let requests = handle.join().unwrap();
        assert_eq!(requests.len(), 2);
        assert!(requests.iter().all(|r| r.starts_with("GET ")));
        let app = serde_json::to_value(app).unwrap();
        let app = app.as_str().unwrap();
        assert!(requests[0].starts_with(&format!("GET /v1/agent-apps/{app}/aggregation/state?targetHostId=host+%2F%3F%E4%B8%AD%E6%96%87&projectId=project+%2F%3F%E4%B8%AD%E6%96%87 ")));
        assert!(
            requests[0]
                .to_lowercase()
                .contains("authorization: bearer fixture-management-grant\r\n")
        );
        assert!(
            requests[1].to_lowercase().contains("authorization: bearer fixture-gateway-key\r\n")
        );
        assert!(
            requests[1]
                .starts_with(&format!("GET /v1/app-providers/{app}/catalog/state?targetHostId="))
        );
        assert!(
            requests[1].to_lowercase().contains("x-agent-apps-token: fixture-management-grant\r\n")
        );
    }
}
#[tokio::test]
async fn aggregation_t56_errors_and_scope_boundaries_remain_visible() {
    for status in [401, 403, 409, 422, 502, 503, 504] {
        let (client, handle) = server(vec![(
            status,
            serde_json::json!({"error":{"code":"fixture","message":"sensitive-payload"}}),
        )]);
        let selection = selection();
        let error =
            load(client, selection.clone(), "fixture-management-grant".into()).await.unwrap_err();
        assert!(
            matches!(error, AggregationApiError::Http { status: actual, .. } if actual == status)
        );
        let mut form = Form::new(selection.clone(), 3);
        assert!(form.finish(&selection, 3, Err(error)));
        assert!(!form.loading);
        assert!(form.snapshot.is_none());
        assert!(!form.error.unwrap().contains("sensitive-payload"));
        assert_eq!(handle.join().unwrap().len(), 1);
    }
    let original = selection();
    let mut variants = [original.clone(), original.clone(), original.clone(), original.clone()];
    variants[0].project = "other-project".into();
    variants[1].scope.host = "other-host".into();
    variants[2].catalog_generation += 1;
    variants[3].scope.context.gateway = "http://other:2".into();
    for changed in variants {
        let mut form = Form::new(changed, 9);
        assert!(!form.finish(
            &original,
            9,
            Ok(Snapshot { state: state(), candidates: Vec::new() })
        ));
        assert!(form.loading);
    }
    let unsupported = AggregationApiError::Http {
        status: 422,
        body: serde_json::json!({"error":{"code":"unsupported_target"}}),
    };
    let invalid = AggregationApiError::Http {
        status: 422,
        body: serde_json::json!({"error":{"code":"invalid_configuration"}}),
    };
    assert!(error_label(&unsupported).contains("不支持"));
    assert!(error_label(&invalid).contains("配置无效"));
    assert!(!supported(AppId::Gemini));
}

#[test]
fn aggregation_t56_scope_change_ends_loading_without_accepting_old_data() {
    let old = selection();
    let mut current = old.clone();
    current.catalog_generation += 1;
    let mut form = Form::new(old.clone(), 1);
    form.invalidate_selection(&current);
    assert!(!form.loading);
    assert!(form.snapshot.is_none());
    assert!(form.error.is_some());
    assert!(!form.finish(&old, 1, Ok(Snapshot { state: state(), candidates: Vec::new() })));
    let mut new = Form::new(current.clone(), 2);
    new.invalidate_selection(&current);
    assert!(new.loading);
    assert!(new.error.is_none());
}
