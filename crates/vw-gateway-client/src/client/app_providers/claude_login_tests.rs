use super::super::test_support;
use vw_api_types::{
    app_providers::{claude::*, claude_login::ClaudeLoginBinding},
    id::TenantScope,
};
#[tokio::test]
async fn cc04_claude_login_sdk_serializes_scoped_binding_and_redacts_errors() {
    let server = test_support::server_with_tenant_scope(
        vec![(403, serde_json::json!({"secret":"must-not-return"}))],
        TenantScope::new("p", "a").unwrap(),
    );
    let error = server
        .client()
        .claude_provider_create(
            "official",
            &ClaudeWrite {
                target_host_id: "host".into(),
                claude: ClaudeProfile {
                    name: "Official".into(),
                    config: ClaudeConfig::Official {
                        account: Some(ClaudeLoginBinding { account_id: "existing".into() }),
                    },
                    model: None,
                },
                activation: ClaudeActivation::Switch,
            },
            "fixture-authorization",
        )
        .await
        .unwrap_err();
    assert!(!error.to_string().contains("must-not-return"));
    let request = server.take_request();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/app-providers/claude/official");
    assert_eq!(request.body["claude"]["config"]["account"]["accountId"], "existing");
    assert_eq!(request.body["activation"], "switch");
    server.join();
}
