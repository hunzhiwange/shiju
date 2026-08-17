#[test]
fn runtime_defaults_match_native_and_docker_profiles() {
    let runtime = super::RuntimeConfig::default();
    assert_eq!(runtime.kind, "native");
    assert_eq!(runtime.docker.image, "alpine:3.20");
    assert_eq!(runtime.docker.network, "none");
    assert_eq!(runtime.docker.memory_limit_mb, Some(512));
}

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
