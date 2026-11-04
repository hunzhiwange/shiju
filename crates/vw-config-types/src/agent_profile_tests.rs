use crate::agent::AgentDefinitionConfig;
use serde_json::json;

#[test]
fn agent_profile_roundtrip_distinguishes_inherit_and_empty() {
    let input = json!({
        "provider": "", "model": "",
        "runtime": {"kind": "acp", "backend": "codex"},
        "recommended_prompts": ["检查代码"],
        "defaults": {
            "model": {"mode": "fixed", "value": {"provider": "openai", "model": "test"}},
            "permission": {"mode": "fixed", "value": {"*": "deny", "file_read": "allow"}},
            "skills": {"mode": "fixed", "value": []},
            "mcp": {"mode": "inherit"}
        }
    });
    let config: AgentDefinitionConfig = serde_json::from_value(input.clone()).unwrap();
    let output = serde_json::to_value(&config).unwrap();
    for key in ["runtime", "recommended_prompts", "defaults"] {
        assert_eq!(output[key], input[key], "{key}");
    }
    let again: AgentDefinitionConfig = serde_json::from_value(output.clone()).unwrap();
    assert_eq!(serde_json::to_value(again).unwrap(), output);
}

#[test]
fn agent_profile_rejects_unknown_runtime() {
    for runtime in [
        json!({"kind": "unknown"}),
        json!({"kind": "acp"}),
        json!({"kind": "native", "backend": "codex"}),
        json!({"kind": "acp", "backend": " "}),
    ] {
        assert!(
            serde_json::from_value::<AgentDefinitionConfig>(json!({
                "provider": "", "model": "", "runtime": runtime
            }))
            .is_err(),
            "{runtime}"
        );
    }
}

#[test]
fn agent_profile_defaults_native_and_inherits() {
    let config: AgentDefinitionConfig =
        serde_json::from_value(json!({"provider": "", "model": ""})).unwrap();
    let value = serde_json::to_value(config).unwrap();
    assert_eq!(value["runtime"], json!({"kind": "native"}));
    for key in ["model", "permission", "skills", "mcp"] {
        assert_eq!(value["defaults"][key], json!({"mode": "inherit"}));
    }
}

#[test]
fn agent_profile_fixed_values_and_invalid_shapes() {
    use crate::agent_profile::AgentDefaults;
    let defaults: AgentDefaults = serde_json::from_value(json!({
        "skills": {"mode": "fixed", "value": ["skill-id"]},
        "mcp": {"mode": "fixed", "value": []}
    }))
    .unwrap();
    let value = serde_json::to_value(defaults).unwrap();
    assert_eq!(value["skills"]["value"], json!(["skill-id"]));
    assert_eq!(value["mcp"]["value"], json!([]));
    for invalid in [
        json!({"model": {"mode": "fixed", "value": {"provider": "p"}}}),
        json!({"model": {"mode": "fixed", "value": {"provider": "", "model": "m"}}}),
        json!({"skills": {"mode": "fixed"}}),
        json!({"skills": {"mode": "inherit", "value": []}}),
    ] {
        assert!(serde_json::from_value::<AgentDefaults>(invalid).is_err());
    }
}

#[test]
fn agent_profile_debug_hides_sensitive_payloads() {
    let config: AgentDefinitionConfig = serde_json::from_value(json!({
        "provider": "", "model": "", "api_key": "secret-key-marker",
        "recommended_prompts": ["private-prompt-marker"],
        "defaults": {"permission": {"mode": "fixed", "value": {"private-path-marker": "deny"}}}
    }))
    .unwrap();
    let debug = format!("{config:?}");
    for secret in ["secret-key-marker", "private-prompt-marker", "private-path-marker"] {
        assert!(!debug.contains(secret));
    }
}

#[test]
fn agent_profile_builtin_merge_preserves_defaults_and_runtime_binding() {
    let config: AgentDefinitionConfig = serde_json::from_value(json!({
        "provider": "", "model": "", "runtime": {"kind": "acp", "backend": "codex"},
        "recommended_prompts": ["检查"], "defaults": {"skills": {"mode": "fixed", "value": []}}
    }))
    .unwrap();
    let merged = crate::agent::merged_agent_configs(&std::collections::HashMap::from([(
        "main".into(),
        config.clone(),
    )]));
    assert_eq!(merged["main"].runtime, crate::agent_profile::AgentRuntime::Native);
    assert_eq!(merged["main"].defaults, config.defaults);
    assert_eq!(merged["main"].recommended_prompts, config.recommended_prompts);
}
