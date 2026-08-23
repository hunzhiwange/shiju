//! 验证本地与容器运行时的默认配置以及推理选项反序列化。
//! 测试区分执行隔离默认值与模型推理覆盖项，避免两类配置相互影响。

// 容器默认禁网并限制内存，本地运行时仍是空配置的首选执行方式。
#[test]
fn runtime_defaults_match_native_and_docker_profiles() {
    let runtime = super::RuntimeConfig::default();
    assert_eq!(runtime.kind, "native");
    assert_eq!(runtime.docker.image, "alpine:3.20");
    assert_eq!(runtime.docker.network, "none");
    assert_eq!(runtime.docker.memory_limit_mb, Some(512));
}

// 可选推理字段只有显式提供时才覆盖上层模型设置。
#[test]
fn runtime_deserializes_reasoning_values() {
    let parsed: super::RuntimeConfig = serde_json::from_value(serde_json::json!({
        "reasoning_enabled": true,
        "reasoning_level": "high"
    }))
    .unwrap();

    assert_eq!(parsed.reasoning_enabled, Some(true));
    assert_eq!(parsed.reasoning_level.as_deref(), Some("high"));
}
