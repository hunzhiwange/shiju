//! 验证设置补丁的稀疏序列化与安全策略协议名称。

use crate::settings::{
    AutonomyLevelDto, NetworkAccessDto, ProxySettingsDto, RuntimeSettingsPatchDto,
    SandboxBackendDto, SecuritySettingsPatchDto,
};
use serde_json::json;

#[test]
fn settings_patches_omit_unset_fields_and_keep_policy_names() {
    let security: SecuritySettingsPatchDto =
        serde_json::from_value(json!({})).expect("valid patch");
    assert_eq!(security.autonomy_level, None);
    assert_eq!(security.network_access, None);

    let runtime = RuntimeSettingsPatchDto { sandbox_backend: Some(SandboxBackendDto::Docker) };
    assert_eq!(
        serde_json::to_value(runtime).expect("serialize"),
        json!({ "sandbox_backend": "docker" })
    );

    assert!(!ProxySettingsDto::default().enabled);
    assert_eq!(serde_json::to_value(AutonomyLevelDto::Strict).expect("serialize"), json!("strict"));
    assert_eq!(serde_json::to_value(NetworkAccessDto::Deny).expect("serialize"), json!("deny"));
}
