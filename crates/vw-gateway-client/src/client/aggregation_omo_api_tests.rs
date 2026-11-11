use super::{AggregationOmoError, GatewayClient, test_support};
use crate::GatewayAuth;
use serde_json::{Value, json};
use vw_api_types::{
    agent_apps::aggregation::{OmoApplyRequest, OmoDisableRequest},
    id::ProjectId,
};

fn state(slim: bool, current: Option<&str>) -> Value {
    let mut value = json!({"revision":"opaque-revision", "currentProviderId":current,
        "agents":{"helper":{"model":"provider/model"}},"otherFields":{"enabled":true}});
    if !slim {
        value["categories"] = json!({"fast":{"model":"provider/fast"}});
    }
    value
}
fn apply() -> OmoApplyRequest {
    OmoApplyRequest {
        target_host_id: "host /?中文".into(),
        project_id: ProjectId::from("project /?"),
        expected_revision: "opaque-revision".into(),
        provider_id: "saved-provider".into(),
    }
}
fn disable() -> OmoDisableRequest {
    let a = apply();
    OmoDisableRequest {
        target_host_id: a.target_host_id,
        project_id: a.project_id,
        expected_revision: a.expected_revision,
    }
}

#[tokio::test]
async fn aggregation_t54_aggregation_omo_api_preserves_contract() {
    for slim in [false, true] {
        let values = [state(slim, None), state(slim, Some("saved-provider")), state(slim, None)];
        let server = test_support::server(values.iter().map(|v| (200, v.clone())).collect());
        let client = GatewayClient::new(
            server
                .client()
                .endpoint()
                .clone()
                .with_auth(GatewayAuth { skey: Some("test-omo-token".into()) }),
        )
        .unwrap();
        let a = apply();
        let d = disable();
        let actual = if slim {
            let read =
                client.aggregation_omo_slim_state(&a.target_host_id, &a.project_id).await.unwrap();
            assert!(read.current_provider_id.is_none());
            vec![
                serde_json::to_value(read).unwrap(),
                serde_json::to_value(client.aggregation_omo_slim_apply(&a).await.unwrap()).unwrap(),
                serde_json::to_value(client.aggregation_omo_slim_disable(&d).await.unwrap())
                    .unwrap(),
            ]
        } else {
            let read =
                client.aggregation_omo_state(&a.target_host_id, &a.project_id).await.unwrap();
            assert!(read.categories.is_some());
            vec![
                serde_json::to_value(read).unwrap(),
                serde_json::to_value(client.aggregation_omo_apply(&a).await.unwrap()).unwrap(),
                serde_json::to_value(client.aggregation_omo_disable(&d).await.unwrap()).unwrap(),
            ]
        };
        assert_eq!(actual, values);
        let resource = if slim { "omo-slim" } else { "omo" };
        for (method, suffix, body) in [
            ("GET", "", Value::Null),
            ("POST", "/apply", serde_json::to_value(&a).unwrap()),
            ("POST", "/disable", serde_json::to_value(&d).unwrap()),
        ] {
            let request = server.take_request();
            assert_eq!(request.method, method);
            let query = if method == "GET" {
                "?targetHostId=host+%2F%3F%E4%B8%AD%E6%96%87&projectId=project+%2F%3F"
            } else {
                ""
            };
            assert_eq!(request.path, format!("/v1/agent-apps/opencode/{resource}{suffix}{query}"));
            assert_eq!(request.body, body);
            assert_eq!(request.header("authorization"), Some("Bearer test-omo-token"));
            assert!(request.header("x-platform-id").is_some());
            assert!(request.header("x-company-id").is_some());
        }
        server.join();
    }
}

#[tokio::test]
async fn aggregation_t54_errors_are_never_empty_success_or_replayed() {
    for (status, code) in [
        (401, "unauthenticated"),
        (403, "forbidden"),
        (409, "revision_conflict"),
        (409, "variant_conflict"),
        (422, "unsupported_target"),
        (422, "invalid_configuration"),
        (502, "target_failed"),
        (503, "target_unavailable"),
        (504, "target_timeout"),
        (307, "redirect"),
    ] {
        let error = json!({"error":{"code":code,"message":"rejected","details":{"retry":false}}});
        let server = test_support::server(vec![(status, error.clone()); 6]);
        let c = server.client();
        let a = apply();
        let d = disable();
        let results = [
            c.aggregation_omo_state(&a.target_host_id, &a.project_id).await.map(|_| ()),
            c.aggregation_omo_slim_state(&a.target_host_id, &a.project_id).await.map(|_| ()),
            c.aggregation_omo_apply(&a).await.map(|_| ()),
            c.aggregation_omo_slim_apply(&a).await.map(|_| ()),
            c.aggregation_omo_disable(&d).await.map(|_| ()),
            c.aggregation_omo_slim_disable(&d).await.map(|_| ()),
        ];
        for result in results {
            assert_eq!(
                result.unwrap_err(),
                AggregationOmoError::Http { status, body: error.clone() }
            );
        }
        for _ in 0..6 {
            server.take_request();
        }
        server.join();
    }
}

#[tokio::test]
async fn aggregation_t54_malformed_and_unexpected_success_are_rejected() {
    for (status, body) in [
        (200, "null".to_string()),
        (200, "{}".into()),
        (200, "not JSON".into()),
        (202, state(false, None).to_string()),
    ] {
        let server = test_support::server_raw(vec![(status, body)]);
        let result =
            server.client().aggregation_omo_state("host", &ProjectId::from("project")).await;
        assert!(
            result.is_err(),
            "unexpected or malformed response must not become success: {status}"
        );
        server.take_request();
        server.join();
    }
}

#[tokio::test]
async fn aggregation_t54_disconnected_target_is_transport_error() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let client = GatewayClient::new(crate::GatewayEndpoint::new("127.0.0.1", port)).unwrap();
    assert!(matches!(
        client.aggregation_omo_slim_disable(&disable()).await,
        Err(AggregationOmoError::Transport(_))
    ));
}
