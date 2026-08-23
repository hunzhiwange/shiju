//! 验证提供商请求对凭据和默认连接选项的显式处理。

use crate::provider::{AuthKind, ConnectProviderRequest, ListProvidersRequest, ProviderStatus};
use serde_json::json;

// 连接请求不隐式提供凭据或设为默认，同时固定认证和连接状态名称。
#[test]
fn provider_requests_keep_credentials_explicit_and_default_empty() {
    let list: ListProvidersRequest = serde_json::from_value(json!({})).expect("valid list");
    assert_eq!(list.enabled, None);
    assert_eq!(list.configured, None);

    let connect: ConnectProviderRequest = serde_json::from_value(json!({})).expect("valid connect");
    assert!(connect.credentials.is_empty());
    assert!(!connect.set_as_default);

    assert_eq!(serde_json::to_value(AuthKind::ApiKey).expect("serialize"), json!("api_key"));
    assert_eq!(
        serde_json::to_value(ProviderStatus::Disconnected).expect("serialize"),
        json!("disconnected")
    );
}
