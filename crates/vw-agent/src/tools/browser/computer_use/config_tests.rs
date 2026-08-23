//! 验证计算机操作配置的安全默认值及密钥脱敏。
//! 本地端点和有限超时是安全默认值，调试输出不得包含 API 密钥。

use super::*;

#[test]
fn default_is_local_and_bounded() {
    let config = ComputerUseConfig::default();
    assert_eq!(config.endpoint, "http://127.0.0.1:8787/v1/actions");
    assert_eq!(config.timeout_ms, 15_000);
    assert!(!config.allow_remote_endpoint);
    assert!(config.window_allowlist.is_empty());
    assert!(config.api_key.is_none());
    assert_eq!(config.max_coordinate_x, None);
    assert_eq!(config.max_coordinate_y, None);
}

#[test]
fn debug_does_not_expose_api_key() {
    let config = ComputerUseConfig {
        api_key: Some("secret-token".into()),
        allow_remote_endpoint: true,
        window_allowlist: vec!["VibeWindow".into()],
        max_coordinate_x: Some(1920),
        max_coordinate_y: Some(1080),
        ..Default::default()
    };
    let rendered = format!("{config:?}");
    assert!(rendered.contains("ComputerUseConfig"));
    assert!(rendered.contains("allow_remote_endpoint"));
    assert!(rendered.contains("VibeWindow"));
    assert!(rendered.contains("1920"));
    assert!(!rendered.contains("secret-token"));
    assert!(!rendered.contains("api_key"));
}
