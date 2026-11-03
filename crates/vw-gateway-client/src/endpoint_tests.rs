//! 验证网关端点的默认租户与链式租户替换行为。
//!
//! 这些测试锁定客户端构造阶段的作用域来源，不涉及实际网络连接。

use vw_api_types::id::{DEFAULT_COMPANY_ID, DEFAULT_PLATFORM_ID, TenantScope};

#[test]
fn gateway_endpoint_defaults_to_platform_tenant() {
    let endpoint = super::GatewayEndpoint::new("127.0.0.1", 8080);

    assert_eq!(endpoint.tenant_scope().platform_id().as_ref(), DEFAULT_PLATFORM_ID);
    assert_eq!(endpoint.tenant_scope().company_id().as_ref(), DEFAULT_COMPANY_ID);
}

#[test]
fn gateway_endpoint_switches_tenant_scope_with_chainable_builder() {
    let tenant_scope = TenantScope::new("platform-east", "company-blue").expect("tenant scope");

    let endpoint = super::GatewayEndpoint::new("gateway.internal", 8080)
        .with_tenant_scope(tenant_scope.clone());

    assert_eq!(endpoint.tenant_scope(), &tenant_scope);
}

#[test]
fn loopback_detection_excludes_remote_and_lookalike_hosts() {
    for host in ["", "localhost", "LOCALHOST", "127.0.0.1", "127.0.0.2", "::1", "[::1]"] {
        assert!(super::GatewayEndpoint::new(host, 42617).is_loopback(), "{host}");
    }
    for host in ["localhost.example.com", "192.168.1.1", "example.com", "::"] {
        assert!(!super::GatewayEndpoint::new(host, 42617).is_loopback(), "{host}");
    }
}
