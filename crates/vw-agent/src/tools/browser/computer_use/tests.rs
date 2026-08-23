//! 验证计算机操作客户端构造和克隆时的安全配置保留。
//! 构造与克隆都必须保留策略对象、域名白名单、会话和坐标限制。

use super::*;
use crate::app::agent::security::SecurityPolicy;
use std::sync::Arc;

#[test]
fn new_preserves_security_boundary_inputs() {
    let client = ComputerUseClient::new(
        Arc::new(SecurityPolicy::default()),
        vec!["example.com".to_string()],
        Some("session-a".to_string()),
        ComputerUseConfig { timeout_ms: 42, ..Default::default() },
    );

    assert_eq!(client.allowed_domains, vec!["example.com"]);
    assert_eq!(client.session_name.as_deref(), Some("session-a"));
    assert_eq!(client.config.timeout_ms, 42);
}

#[test]
fn cloned_client_preserves_configuration() {
    let client = ComputerUseClient::new(
        Arc::new(SecurityPolicy::default()),
        vec!["example.com".to_string(), "docs.example.com".to_string()],
        None,
        ComputerUseConfig {
            endpoint: "http://127.0.0.1:7777/actions".into(),
            timeout_ms: 99,
            ..Default::default()
        },
    );

    let cloned = client.clone();

    assert_eq!(cloned.allowed_domains, client.allowed_domains);
    assert_eq!(cloned.session_name, None);
    assert_eq!(cloned.config.endpoint, "http://127.0.0.1:7777/actions");
    assert_eq!(cloned.config.timeout_ms, 99);
}
