use super::{AgentAppsHighAvailabilityError as Error, GatewayClient, test_support};
use crate::endpoint::GatewayAuth;
use serde_json::json;
use vw_api_types::{
    agent_apps::AppId,
    agent_apps_high_availability::*,
    id::{ProjectScope, TenantScope},
};

#[tokio::test]
async fn ha_sdk_preserves_scope_and_does_not_retry_writes() {
    let tenant = TenantScope::new("platform", "company").unwrap();
    let query = FailoverQueueQuery {
        project_id: "project /?".into(), target_host_id: "host /&".into(), app_id: AppId::Codex,
    };
    let scope = FailoverQueueScope {
        project: ProjectScope::new(tenant.clone(), query.project_id.clone()).unwrap(),
        target_host_id: query.target_host_id.clone(), app_id: query.app_id,
    };
    let queue = FailoverQueueResponse {
        scope: scope.clone(), version: FailoverQueueVersion(7), enabled: false,
        entries: vec![FailoverQueueEntry { provider_id: "provider /&".into(), name: "备用".into() }],
    };
    let candidates = FailoverCandidatesResponse {
        scope: scope.clone(), candidates: queue.entries.clone(),
    };
    let health = FailoverHealthResponse {
        scope: scope.clone(), provider_id: "provider /&".into(),
        state: FailoverHealthState::Open, retry_after_ms: Some(123),
    };
    let route = FailoverRouteResponse {
        scope, configured_provider_id: Some("configured".into()),
        actual_provider_id: Some("actual".into()),
        failure_reason: Some(FailoverFailureReason::RateLimited),
    };
    let policy = FailoverPolicy {
        circuit_breaker: CircuitBreakerPolicy {
            failure_threshold: 3, success_threshold: 2, cooldown_secs: 30,
            error_rate_threshold: 0.5, min_requests: 5,
        },
        retry_timeout: RetryTimeoutPolicy {
            max_retries: 2, first_byte_timeout_secs: 30,
            idle_timeout_secs: 60, non_streaming_timeout_secs: 300,
        },
    };
    let server = test_support::server_with_tenant_scope(vec![
        (200, json!(queue)), (200, json!(candidates)),
        (200, json!(queue)), (200, json!(queue)), (200, json!(queue)), (200, json!(queue)),
        (200, json!(health)), (200, json!(health)), (200, json!(route)),
        (200, json!(policy)), (200, json!(policy)),
    ], tenant);
    let client = GatewayClient::new(server.client().endpoint().clone().with_auth(
        GatewayAuth { skey: Some("gateway-token".into()) },
    )).unwrap();
    let token = Some("application-grant");
    let member = FailoverQueueMemberRequest {
        expected_version: queue.version, provider_id: health.provider_id.clone(),
    };
    let reorder = FailoverQueueReorderRequest {
        expected_version: queue.version, provider_ids: vec![health.provider_id.clone()],
    };
    let enabled = FailoverQueueEnabledRequest { expected_version: queue.version, enabled: false };
    assert_eq!(client.agent_apps_failover_queue(&query, token).await.unwrap(), queue);
    assert_eq!(client.agent_apps_failover_candidates(&query, token).await.unwrap(), candidates);
    assert_eq!(client.agent_apps_failover_add_member(&query, &member, token).await.unwrap(), queue);
    assert_eq!(client.agent_apps_failover_remove_member(&query, &member, token).await.unwrap(), queue);
    assert_eq!(client.agent_apps_failover_reorder(&query, &reorder, token).await.unwrap(), queue);
    assert_eq!(client.agent_apps_failover_set_enabled(&query, &enabled, token).await.unwrap(), queue);
    assert_eq!(client.agent_apps_failover_health(&query, &health.provider_id, token).await.unwrap(), health);
    assert_eq!(client.agent_apps_failover_reset_health(&query, &health.provider_id, token).await.unwrap(), health);
    assert_eq!(client.agent_apps_failover_route(&query, token).await.unwrap(), route);
    assert_eq!(client.agent_apps_failover_policy(&query, token).await.unwrap(), policy);
    assert_eq!(client.agent_apps_failover_save_policy(&query, &policy, token).await.unwrap(), policy);
    for (method, resource, body, provider) in [
        ("GET", "queue", json!(null), false),
        ("GET", "candidates", json!(null), false),
        ("POST", "queue/members", json!(member), false),
        ("DELETE", "queue/members", json!(member), false),
        ("PUT", "queue/order", json!(reorder), false),
        ("PUT", "enabled", json!(enabled), false),
        ("GET", "health", json!(null), true),
        ("POST", "health/reset", json!(null), true),
        ("GET", "route", json!(null), false),
        ("GET", "policy", json!(null), false),
        ("PUT", "policy", json!(policy), false),
    ] {
        let request = server.take_request();
        assert_eq!(request.method, method);
        let suffix = if provider { "&providerId=provider+%2F%26" } else { "" };
        assert_eq!(request.path, format!(
            "/v1/agent-apps/high-availability/{resource}?projectId=project+%2F%3F&targetHostId=host+%2F%26&appId=codex{suffix}"
        ));
        assert_eq!(request.body, body);
        assert_eq!(request.header("authorization"), Some("Bearer gateway-token"));
        assert_eq!(request.header("x-agent-apps-token"), token);
        assert_eq!(request.header("x-platform-id"), Some("platform"));
        assert_eq!(request.header("x-company-id"), Some("company"));
    }
    server.join();

    for status in [403, 409, 503, 307] {
        let body = json!({"error": {"code": "version_conflict", "message": "configuration changed",
            "details": {"correlationId": "request-id"}}});
        // Each failed write receives exactly one response. A replay would fail to
        // connect after the server exits, losing the original status/body assertion.
        let server = test_support::server(vec![(status, body.clone()); 6]);
        let client = server.client();
        let errors = [
            client.agent_apps_failover_add_member(&query, &member, None).await.unwrap_err(),
            client.agent_apps_failover_remove_member(&query, &member, None).await.unwrap_err(),
            client.agent_apps_failover_reorder(&query, &reorder, None).await.unwrap_err(),
            client.agent_apps_failover_set_enabled(&query, &enabled, None).await.unwrap_err(),
            client.agent_apps_failover_reset_health(&query, &health.provider_id, None).await.unwrap_err(),
            client.agent_apps_failover_save_policy(&query, &policy, None).await.unwrap_err(),
        ];
        for error in errors {
            assert_eq!(error, Error::Http {
                status, body: Some(serde_json::from_value(body.clone()).unwrap()),
            });
            assert!(!error.to_string().contains("request-id"));
            assert_eq!(server.take_request().header("x-agent-apps-token"), None);
        }
        server.join();
    }
}

#[tokio::test]
async fn ha_sdk_discards_unstructured_error_payload() {
    let server = test_support::server_raw(vec![(503, "sensitive upstream payload".into())]);
    let query = FailoverQueueQuery {
        project_id: "project".into(), target_host_id: "host".into(), app_id: AppId::Codex,
    };
    assert_eq!(server.client().agent_apps_failover_queue(&query, None).await.unwrap_err(),
        Error::Http { status: 503, body: None });
    server.take_request();
    server.join();
}
