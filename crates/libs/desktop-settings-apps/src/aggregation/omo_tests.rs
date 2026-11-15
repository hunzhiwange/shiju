use desktop_apps_ui::aggregation::omo_state::*;
use gateway_aggregation_omo::AggregationOmoError;
use vw_api_app_aggregation::*;
fn standard(revision: &str, current: Option<&str>) -> Snapshot {
    Snapshot::Standard(OmoState {
        revision: revision.into(),
        current_provider_id: current.map(Into::into),
        agents: None,
        categories: Some(Default::default()),
        other_fields: None,
    })
}
#[test]
fn aggregation_t64_aggregation_omo_preserves_contract() {
    let mut form = Form::new(Variant::Standard);
    form.provider = "draft".into();
    form.finish(Ok(standard("r1", Some("p1"))));
    assert_eq!(form.snapshot.as_ref().unwrap().current(), Some("p1"));
    assert!(form.snapshot.as_ref().unwrap().sections().iter().any(|(s, _)| *s == "Categories"));
    form.finish(Err(AggregationOmoError::Http {
        status: 409,
        body: serde_json::json!({"error":{"code":"revision_conflict"}}),
    }));
    assert!(form.snapshot.is_none(), "stale revision must not enable another write");
    assert_eq!(form.provider, "draft");
    assert!(!form.busy);
    form.finish(Ok(standard("r2", None)));
    assert_eq!(form.snapshot.as_ref().unwrap().revision(), "r2");
    assert_eq!(form.snapshot.as_ref().unwrap().current(), None);
    assert!(form.error.is_none());
}
#[test]
fn aggregation_t64_slim_has_no_categories_and_jsonc_error_is_retryable() {
    let mut form = Form::new(Variant::Slim);
    form.finish(Ok(Snapshot::Slim(OmoSlimState {
        revision: "s1".into(),
        current_provider_id: None,
        agents: Some(Default::default()),
        other_fields: None,
    })));
    assert!(!form.snapshot.as_ref().unwrap().sections().iter().any(|(s, _)| *s == "Categories"));
    form.finish(Err(AggregationOmoError::Http {
        status: 422,
        body: serde_json::json!({"secret":"must never display"}),
    }));
    assert!(!form.busy);
    assert!(!form.error.as_ref().unwrap().contains("secret"));
    assert!(form.error.as_ref().unwrap().contains("刷新"));
}

#[tokio::test]
async fn aggregation_t64_sdk_uses_variant_scope_revision_and_provider_reference() {
    use super::{Action, Context, request};
    use desktop_apps_ui::{AgentAppsContext, ProviderScope as Scope};
    use std::io::{Read, Write};
    use {gateway_endpoint::{GatewayAuth, GatewayEndpoint}, gateway_aggregation_omo::GatewayAggregationOmoClient, vw_api_app_identity::AppId};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let worker = std::thread::spawn(move || {
        let mut captured = vec![];
        for path in
            ["omo", "omo/apply", "omo/disable", "omo-slim", "omo-slim/apply", "omo-slim/disable"]
        {
            let (mut stream, _) = listener.accept().unwrap();
            stream.set_read_timeout(Some(std::time::Duration::from_secs(10))).unwrap();
            let mut bytes = vec![];
            let mut chunk = [0; 4096];
            loop {
                let n = stream.read(&mut chunk).unwrap();
                assert!(n > 0);
                bytes.extend_from_slice(&chunk[..n]);
                if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&bytes[..end]).to_lowercase();
                    let length = headers
                        .lines()
                        .find_map(|l| {
                            l.strip_prefix("content-length:")
                                .map(|v| v.trim().parse::<usize>().unwrap())
                        })
                        .unwrap_or(0);
                    if bytes.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            let input = String::from_utf8(bytes).unwrap();
            assert!(
                input.lines().next().unwrap().contains(&format!("/v1/agent-apps/opencode/{path}"))
            );
            assert!(input.to_lowercase().contains("authorization: bearer fixture-t64"));
            captured.push(input);
            let result = if path.ends_with("disable") {
                serde_json::json!({"revision":"r3","currentProviderId":null})
            } else {
                serde_json::json!({"revision":"r2","currentProviderId":"saved"})
            };
            let body = result.to_string();
            write!(stream,"HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{}",body.len(),body).unwrap();
        }
        captured
    });
    let client = GatewayAggregationOmoClient::new(gateway_http::GatewayTransport::new(
        GatewayEndpoint::new("127.0.0.1", port)
            .with_auth(GatewayAuth { skey: Some("fixture-t64".into()) }),
    ).unwrap());
    let context = Context {
        scope: Scope {
            context: AgentAppsContext {
                gateway: client.endpoint().base_url(),
                tenant: Default::default(),
                project: Some("/fixture".into()),
            },
            host: "host-fixture".into(),
            app: AppId::OpenCode,
        },
        project: "project-fixture".into(),
        access: 1,
    };
    for variant in [Variant::Standard, Variant::Slim] {
        for action in [Action::Read, Action::Apply("saved".into()), Action::Disable] {
            let disabled = action == Action::Disable;
            let value = request(client.clone(), context.clone(), variant, action, "r1".into())
                .await
                .unwrap();
            assert_eq!(value.current(), if disabled { None } else { Some("saved") });
        }
    }
    for (n, input) in worker.join().unwrap().iter().enumerate() {
        if n % 3 == 0 {
            assert!(input.contains("projectId=project-fixture"));
            assert!(input.contains("targetHostId=host-fixture"));
        } else {
            let body: serde_json::Value =
                serde_json::from_str(input.split_once("\r\n\r\n").unwrap().1).unwrap();
            assert_eq!(body["projectId"], "project-fixture");
            assert_eq!(body["targetHostId"], "host-fixture");
            assert_eq!(body["expectedRevision"], "r1");
            assert_eq!(
                body.get("providerId").and_then(|v| v.as_str()),
                if n % 3 == 1 { Some("saved") } else { None }
            );
            assert!(body.get("categories").is_none());
            assert!(body.get("agents").is_none());
        }
    }
}
