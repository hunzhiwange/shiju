use super::{GatewayClient, OpenClawSettingsError as Error, OpenClawSettingsScope, test_support};
use serde_json::json;
use vw_api_types::{
    id::{ProjectId, TenantScope},
    openclaw_settings::*,
};

fn scope() -> OpenClawSettingsScope {
    OpenClawSettingsScope {
        project_id: ProjectId::new("project").unwrap(),
        target_host_id: "主机 /?".into(),
    }
}

#[tokio::test]
async fn openclaw_sdk_preserves_warnings_and_sections() {
    let warning = json!({"code":"configuration_write","message":"review configuration","path":"agents.defaults.model"});
    let outcome = json!({"backupPath":"backup/config.json5","warnings":[warning.clone()]});
    let fixtures = [
        (
            "model",
            json!({"primary":"provider/model","fallbacks":["other/model"],"future":{"enabled":true}}),
        ),
        ("catalog", json!({"provider/model":{"alias":"中文","future":[1,2]}})),
        ("defaults", json!({"model":{"primary":"provider/model"},"future":7})),
        ("env", json!({"vars":{"TOKEN":"private"},"shellEnv":{"enabled":true},"future":false})),
        (
            "tools",
            json!({"profile":"unknown-future-profile","allow":["read"],"deny":["exec"],"future":{}}),
        ),
    ];
    let mut responses = Vec::new();
    for (_, value) in &fixtures {
        responses.extend([(200, value.clone()), (200, outcome.clone()), (200, json!(null))]);
    }
    responses.push((200, json!([warning])));
    let server = test_support::server_with_tenant_scope(
        responses,
        TenantScope::new("platform", "company").unwrap(),
    );
    let client = server.client();
    let s = scope();
    let value = client.openclaw_settings_model(&s, "grant").await.unwrap().unwrap();
    assert_eq!(serde_json::to_value(&value).unwrap(), fixtures[0].1);
    let saved = client.set_openclaw_settings_model(&s, &value, "grant").await.unwrap();
    assert_eq!(serde_json::to_value(saved).unwrap(), outcome);
    assert!(client.openclaw_settings_model(&s, "grant").await.unwrap().is_none());
    let value = client.openclaw_settings_catalog(&s, "grant").await.unwrap().unwrap();
    assert_eq!(serde_json::to_value(&value).unwrap(), fixtures[1].1);
    let saved = client.set_openclaw_settings_catalog(&s, &value, "grant").await.unwrap();
    assert_eq!(serde_json::to_value(saved).unwrap(), outcome);
    assert!(client.openclaw_settings_catalog(&s, "grant").await.unwrap().is_none());
    let value = client.openclaw_settings_defaults(&s, "grant").await.unwrap().unwrap();
    assert_eq!(serde_json::to_value(&value).unwrap(), fixtures[2].1);
    let saved = client.set_openclaw_settings_defaults(&s, &value, "grant").await.unwrap();
    assert_eq!(serde_json::to_value(saved).unwrap(), outcome);
    assert!(client.openclaw_settings_defaults(&s, "grant").await.unwrap().is_none());
    let value = client.openclaw_settings_env(&s, "grant").await.unwrap().unwrap();
    assert_eq!(serde_json::to_value(&value).unwrap(), fixtures[3].1);
    let saved = client.set_openclaw_settings_env(&s, &value, "grant").await.unwrap();
    assert_eq!(serde_json::to_value(saved).unwrap(), outcome);
    assert!(client.openclaw_settings_env(&s, "grant").await.unwrap().is_none());
    let value = client.openclaw_settings_tools(&s, "grant").await.unwrap().unwrap();
    assert_eq!(serde_json::to_value(&value).unwrap(), fixtures[4].1);
    let saved = client.set_openclaw_settings_tools(&s, &value, "grant").await.unwrap();
    assert_eq!(serde_json::to_value(saved).unwrap(), outcome);
    assert!(client.openclaw_settings_tools(&s, "grant").await.unwrap().is_none());
    assert_eq!(
        serde_json::to_value(client.openclaw_settings_health(&s, "grant").await.unwrap()).unwrap(),
        json!([warning])
    );
    for (section, value) in fixtures {
        for (method, body) in [("GET", json!(null)), ("PUT", value), ("GET", json!(null))] {
            let request = server.take_request();
            assert_eq!(request.method, method);
            assert_eq!(
                request.path,
                format!(
                    "/v1/agent-apps/openclaw/settings/{section}?projectId=project&targetHostId=%E4%B8%BB%E6%9C%BA+%2F%3F"
                )
            );
            assert_eq!(request.header("x-platform-id"), Some("platform"));
            assert_eq!(request.header("x-company-id"), Some("company"));
            assert_eq!(request.header("x-agent-apps-token"), Some("grant"));
            assert_eq!(request.body, body);
        }
    }
    assert!(server.take_request().path.starts_with("/v1/agent-apps/openclaw/settings/health?"));
    server.join();
}

#[tokio::test]
async fn openclaw_sdk_preserves_warnings_rejections_and_decode_errors() {
    for status in [401, 403, 404, 409, 422, 500, 503] {
        let server =
            test_support::server_raw(vec![(status, "private".into()), (status, "private".into())]);
        assert!(
            matches!(server.client().openclaw_settings_env(&scope(), "grant").await, Err(Error::Http(s)) if s == status)
        );
        assert_eq!(
            server
                .client()
                .set_openclaw_settings_tools(&scope(), &OpenClawToolsConfig::default(), "grant")
                .await,
            Err(Error::Http(status))
        );
        server.join();
    }
    let server = test_support::server(vec![(200, json!({})), (200, json!({})), (200, json!({}))]);
    let client = server.client();
    assert!(matches!(client.openclaw_settings_model(&scope(), "grant").await, Err(Error::Decode)));
    assert_eq!(
        client
            .set_openclaw_settings_tools(&scope(), &OpenClawToolsConfig::default(), "grant")
            .await,
        Err(Error::Decode)
    );
    assert!(client.openclaw_settings_env(&scope(), "grant").await.unwrap().unwrap().0.is_empty());
    server.join();
}

#[tokio::test]
async fn openclaw_sdk_preserves_warnings_invalid_input_and_transport() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let client =
        GatewayClient::new(crate::endpoint::GatewayEndpoint::new("127.0.0.1", port)).unwrap();
    for token in ["", "bad\ncredential"] {
        assert_eq!(
            client.openclaw_settings_health(&scope(), token).await,
            Err(Error::InvalidInput)
        );
    }
    let mut s = scope();
    s.target_host_id.clear();
    assert_eq!(client.openclaw_settings_health(&s, "grant").await, Err(Error::InvalidInput));
    assert_eq!(client.openclaw_settings_health(&scope(), "grant").await, Err(Error::Transport));
}
