use std::path::Path;
use std::{collections::HashMap, path::PathBuf};

#[test]
fn workspaces_config_resolves_default_and_relative_roots() {
    let config_dir = Path::new("/tmp/vw");

    let default_root = super::WorkspacesConfig::default().resolve_root(config_dir);
    assert_eq!(default_root, config_dir.join("workspaces"));

    let relative = super::WorkspacesConfig { enabled: true, root: Some("custom".into()) }
        .resolve_root(config_dir);
    assert_eq!(relative, config_dir.join("custom"));
}

#[test]
fn agent_definition_defaults_and_builtin_specs_are_stable() {
    let definition = super::AgentDefinitionConfig::default();
    assert_eq!(definition.mode, "all");
    assert!(definition.enabled);
    assert!(definition.allow_as_subagent);
    assert!(definition.agentic);
    assert_eq!(definition.max_parallel_subagents, 4);
    assert_eq!(definition.max_depth, 3);
    assert_eq!(definition.max_iterations, 50_000);
    assert!(!definition.compact_context);
    assert_eq!(definition.max_history_messages, 100);
    assert!(!definition.parallel_tools);

    assert_eq!(super::BUILTIN_AGENT_SPECS.len(), 5);
    assert_eq!(super::BUILTIN_AGENT_SPECS[0].key, "main");
}

#[test]
fn agent_definition_defaults_include_subagent_controls() {
    let definition: super::AgentDefinitionConfig = serde_json::from_value(serde_json::json!({
        "provider": "openai",
        "model": "gpt-test"
    }))
    .expect("legacy-shaped agent config should deserialize");

    assert!(definition.allow_as_subagent);
    assert!(definition.agentic);
    assert_eq!(definition.max_parallel_subagents, 4);

    let value = serde_json::to_value(&definition).expect("agent config should serialize");
    assert_eq!(value["allow_as_subagent"], true);
    assert_eq!(value["max_parallel_subagents"], 4);
    assert_eq!(value["compact_context"], false);
    assert_eq!(value["max_history_messages"], 100);
    assert_eq!(value["parallel_tools"], false);
}

#[test]
fn delegate_capability_policies_default_to_unrestricted_and_round_trip() {
    let legacy: super::AgentDefinitionConfig = serde_json::from_value(serde_json::json!({
        "provider": "openai",
        "model": "gpt-test"
    }))
    .expect("legacy-shaped agent config should deserialize");

    assert_eq!(legacy.tool_policy, super::DelegateCapabilityPolicy::Unrestricted);
    assert_eq!(legacy.skill_policy, super::DelegateCapabilityPolicy::Unrestricted);
    assert_eq!(
        super::AgentDefinitionConfig::default().tool_policy,
        super::DelegateCapabilityPolicy::Unrestricted
    );
    assert_eq!(
        super::AgentDefinitionConfig::default().skill_policy,
        super::DelegateCapabilityPolicy::Unrestricted
    );

    let definition = super::AgentDefinitionConfig {
        provider: "openai".into(),
        model: "gpt-test".into(),
        tool_policy: super::DelegateCapabilityPolicy::Strict,
        skill_policy: super::DelegateCapabilityPolicy::UserOverride,
        ..Default::default()
    };
    let value = serde_json::to_value(&definition).expect("agent config should serialize");
    assert_eq!(value["tool_policy"], "strict");
    assert_eq!(value["skill_policy"], "user_override");

    let policy_schema =
        serde_json::to_value(schemars::schema_for!(super::DelegateCapabilityPolicy))
            .expect("capability policy schema should serialize");
    assert_eq!(
        policy_schema["enum"],
        serde_json::json!(["strict", "user_override", "unrestricted"])
    );

    let decoded: super::AgentDefinitionConfig =
        serde_json::from_value(value).expect("agent config should deserialize");
    assert_eq!(decoded.tool_policy, super::DelegateCapabilityPolicy::Strict);
    assert_eq!(decoded.skill_policy, super::DelegateCapabilityPolicy::UserOverride);

    let error = serde_json::from_value::<super::AgentDefinitionConfig>(serde_json::json!({
        "provider": "openai",
        "model": "gpt-test",
        "tool_policy": "unknown"
    }))
    .expect_err("invalid capability policy must fail to deserialize");
    assert!(error.to_string().contains("unknown variant"));
}

#[test]
fn agent_definition_force_model_selection_defaults_off() {
    let definition: super::AgentDefinitionConfig = serde_json::from_value(serde_json::json!({
        "provider": "openai",
        "model": "gpt-test"
    }))
    .expect("legacy-shaped agent config should deserialize");

    assert!(!definition.force_model_selection);
}

#[test]
fn agent_definition_force_model_selection_round_trips() {
    let definition = super::AgentDefinitionConfig {
        provider: "openai".into(),
        model: "gpt-test".into(),
        force_model_selection: true,
        ..Default::default()
    };

    let value = serde_json::to_value(&definition).expect("agent config should serialize");
    assert_eq!(value["force_model_selection"], true);

    let decoded: super::AgentDefinitionConfig =
        serde_json::from_value(value).expect("agent config should deserialize");
    assert!(decoded.force_model_selection);
}

#[test]
fn agent_definition_complete_model_binding_requires_non_blank_provider_and_model() {
    let complete = super::AgentDefinitionConfig {
        provider: " openai ".into(),
        model: " gpt-test ".into(),
        ..Default::default()
    };
    assert!(complete.has_complete_model_binding());

    for (provider, model) in
        [("", "gpt-test"), (" \t ", "gpt-test"), ("openai", ""), ("openai", " \t ")]
    {
        let incomplete = super::AgentDefinitionConfig {
            provider: provider.into(),
            model: model.into(),
            force_model_selection: true,
            ..Default::default()
        };
        assert!(!incomplete.has_complete_model_binding());
    }
}

#[test]
fn agent_definition_identity_files_default_empty_and_round_trip() {
    let legacy: super::AgentDefinitionConfig = serde_json::from_value(serde_json::json!({
        "provider": "openai",
        "model": "gpt-test"
    }))
    .expect("legacy-shaped agent config should deserialize");

    assert!(legacy.enabled_identity_files.is_empty());

    let definition = super::AgentDefinitionConfig {
        provider: "openai".to_string(),
        model: "gpt-test".to_string(),
        enabled_identity_files: vec!["SOUL.md".to_string(), "AGENTS.md".to_string()],
        ..Default::default()
    };
    let value = serde_json::to_value(&definition).expect("agent config should serialize");
    assert_eq!(value["enabled_identity_files"], serde_json::json!(["SOUL.md", "AGENTS.md"]));
    let decoded: super::AgentDefinitionConfig =
        serde_json::from_value(value).expect("agent config should deserialize");

    assert_eq!(decoded.enabled_identity_files, vec!["SOUL.md", "AGENTS.md"]);
}

#[test]
fn agent_definition_preserves_explicit_subagent_opt_out() {
    let definition: super::AgentDefinitionConfig = serde_json::from_value(serde_json::json!({
        "provider": "openai",
        "model": "gpt-test",
        "allow_as_subagent": false
    }))
    .expect("explicit subagent opt-out should deserialize");

    assert!(!definition.allow_as_subagent);
}

#[test]
fn workspaces_config_handles_blank_and_absolute_root_overrides() {
    let config_dir = Path::new("/tmp/vw");

    let blank = super::WorkspacesConfig { enabled: true, root: Some(" \t ".into()) };
    assert_eq!(blank.resolve_root(config_dir), config_dir.join("workspaces"));

    let absolute = super::WorkspacesConfig {
        enabled: true,
        root: Some("/var/lib/vibewindow/workspaces".into()),
    };
    assert_eq!(absolute.resolve_root(config_dir), PathBuf::from("/var/lib/vibewindow/workspaces"));
}

#[test]
fn builtin_agent_configs_apply_role_specific_permissions() {
    let main = super::builtin_agent_config("main").expect("main preset");
    assert_eq!(main.mode, "primary");
    assert_eq!(main.permission["file_write"], "allow");

    let explore = super::builtin_agent_config("explore").expect("explore preset");
    assert_eq!(explore.mode, "subagent");
    assert_eq!(explore.permission["*"], "deny");
    assert_eq!(explore.permission["file_read"], "allow");
    assert!(!explore.allowed_tools.iter().any(|tool| tool == "bash"));

    for key in ["build", "plan", "compaction", "title", "summary"] {
        assert!(super::builtin_agent_config(key).is_none(), "{key} must not be public");
    }

    assert!(super::builtin_agent_config("unknown").is_none());
}

#[test]
fn builtin_agent_configs_do_not_enable_git_or_semantic_analysis_by_default() {
    for key in super::builtin_agent_keys() {
        let config = super::builtin_agent_config(key).expect("builtin config");
        assert!(
            !config.allowed_tools.iter().any(|tool| tool == "lsp"),
            "{key} must not select semantic analysis by default"
        );
        assert!(
            !config.permission.as_object().is_some_and(|rules| rules.contains_key("lsp")),
            "{key} must not permit semantic analysis by default"
        );
    }

    for key in ["main", "coder"] {
        let config = super::builtin_agent_config(key).expect("builtin config");
        assert!(
            !config.allowed_tools.iter().any(|tool| tool == "git_operations"),
            "{key} must not select Git operations by default"
        );
        assert!(
            !config
                .permission
                .as_object()
                .is_some_and(|rules| rules.contains_key("git_operations")),
            "{key} must not permit Git operations by default"
        );
    }
}

#[test]
fn merged_agent_configs_fill_builtin_metadata_and_preserve_custom_entries() {
    let mut configured = HashMap::new();
    configured.insert(
        "coder".to_string(),
        super::AgentDefinitionConfig {
            provider: "openrouter".into(),
            model: "coder-model".into(),
            ..Default::default()
        },
    );
    configured.insert(
        "custom".to_string(),
        super::AgentDefinitionConfig {
            label: Some("Custom".into()),
            provider: "local".into(),
            model: "custom-model".into(),
            ..Default::default()
        },
    );

    let merged = super::merged_agent_configs(&configured);
    let coder = merged.get("coder").expect("configured builtin");
    assert_eq!(coder.label.as_deref(), Some("代码实现"));
    assert!(coder.description.is_some());
    assert!(coder.builtin);
    assert_eq!(coder.model, "coder-model");

    let custom = merged.get("custom").expect("custom agent");
    assert_eq!(custom.label.as_deref(), Some("Custom"));
    assert!(!custom.builtin);
    assert_eq!(custom.provider, "local");
}

#[test]
fn agent_definition_debug_redacts_api_key_and_includes_runtime_fields() {
    let definition = super::AgentDefinitionConfig {
        label: Some("Sensitive".into()),
        provider: "anthropic".into(),
        model: "claude-test".into(),
        system_prompt: Some("be useful".into()),
        api_key: Some("sk-secret".into()),
        temperature: Some(0.2),
        top_p: Some(0.9),
        identity_format: Some("openclaw".into()),
        hidden: true,
        agentic: true,
        allowed_tools: vec!["read".into()],
        allowed_skills: vec!["rust".into()],
        options: HashMap::from([("effort".into(), serde_json::json!("high"))]),
        permission: serde_json::json!({ "read": "allow" }),
        steps: Some(12),
        ..Default::default()
    };

    let debug = format!("{definition:?}");

    assert!(debug.contains("AgentDefinitionConfig"));
    assert!(debug.contains("api_key_configured: true"));
    assert!(debug.contains("allowed_tools"));
    assert!(debug.contains("allowed_skills"));
    assert!(debug.contains("max_iterations"));
    assert!(!debug.contains("sk-secret"));
}

#[test]
fn builtin_agent_keys_follow_spec_order() {
    let keys = super::builtin_agent_keys();

    assert_eq!(keys, vec!["main", "researcher", "coder", "reviewer", "explore"]);
}

#[test]
fn merged_builtin_overrides_preserve_catalog_metadata_and_mark_builtin() {
    let configured = HashMap::from([(
        "reviewer".to_string(),
        super::AgentDefinitionConfig {
            label: Some("Strict Reviewer".into()),
            description: Some("custom review role".into()),
            provider: "openrouter".into(),
            model: "review-model".into(),
            ..Default::default()
        },
    )]);

    let merged = super::merged_agent_configs(&configured);
    let reviewer = merged.get("reviewer").expect("configured reviewer");

    assert_eq!(reviewer.label.as_deref(), Some("代码审查"));
    assert_eq!(
        reviewer.description.as_deref(),
        Some("只读审查代码、测试与风险，给出有证据的缺陷报告。")
    );
    assert!(reviewer.builtin);
}

#[test]
fn builtin_agent_catalog_uses_chinese_metadata_and_safe_delegate_roles() {
    let keys = super::builtin_agent_keys();
    assert_eq!(keys, vec!["main", "researcher", "coder", "reviewer", "explore"]);

    let expected_delegate_roles = ["researcher", "coder", "reviewer", "explore"];
    for key in &keys {
        let config = super::builtin_agent_config(key).expect("builtin config");
        assert!(config.label.as_deref().is_some_and(|label| !label.is_ascii()));
        assert!(config.description.as_deref().is_some_and(|description| !description.is_ascii()));
        let is_delegate_role = expected_delegate_roles.contains(key);
        assert_eq!(config.allow_as_subagent, is_delegate_role);
        assert_eq!(config.max_depth, u32::from(is_delegate_role));
        if is_delegate_role {
            assert!(!config.allowed_tools.iter().any(|tool| tool == "AgentTool"));
        }
    }

    for key in ["researcher", "reviewer", "explore"] {
        let config = super::builtin_agent_config(key).expect("readonly delegate config");
        assert_eq!(config.temperature, Some(0.1));
        assert_eq!(config.max_iterations, 32);
    }
    let coder = super::builtin_agent_config("coder").expect("coder config");
    assert_eq!(coder.temperature, Some(0.2));
    assert_eq!(coder.max_iterations, 64);
    assert!(super::builtin_agent_config("compaction").is_none());
}

#[test]
fn retired_builtin_agents_cannot_reenter_effective_catalog() {
    let configured = ["build", "plan", "compaction", "title", "summary"]
        .into_iter()
        .map(|key| {
            (
                key.to_string(),
                super::AgentDefinitionConfig {
                    label: Some(format!("legacy-{key}")),
                    provider: "legacy".into(),
                    model: "legacy-model".into(),
                    ..Default::default()
                },
            )
        })
        .collect();

    let merged = super::merged_agent_configs(&configured);
    for key in ["build", "plan", "compaction", "title", "summary"] {
        assert!(!merged.contains_key(key), "{key} must not become a custom agent");
    }
}

#[test]
fn removed_general_agent_cannot_reenter_effective_catalog() {
    let configured = HashMap::from([(
        "general".to_string(),
        super::AgentDefinitionConfig {
            label: Some("旧通用代理".into()),
            provider: "legacy".into(),
            model: "legacy-model".into(),
            ..Default::default()
        },
    )]);

    assert!(super::builtin_agent_config("general").is_none());
    assert!(!super::merged_agent_configs(&configured).contains_key("general"));
}

#[test]
fn configured_builtin_override_cannot_broaden_role_safety_boundary() {
    let configured = HashMap::from([
        (
            "main".to_string(),
            super::AgentDefinitionConfig {
                label: Some("恶意主代理".into()),
                description: Some("恶意描述".into()),
                enabled: true,
                allow_as_subagent: true,
                max_parallel_subagents: 0,
                hidden: true,
                max_depth: 99,
                allowed_tools: vec!["AgentTool".into(), "shell".into()],
                permission: serde_json::json!({"*": "allow"}),
                max_iterations: 777,
                provider: "openrouter".into(),
                model: "main-model".into(),
                system_prompt: Some("覆盖提示词".into()),
                ..Default::default()
            },
        ),
        (
            "reviewer".to_string(),
            super::AgentDefinitionConfig {
                enabled: true,
                allow_as_subagent: true,
                max_depth: 99,
                allowed_tools: vec!["shell".into(), "file_read".into()],
                permission: serde_json::json!({"*": "allow"}),
                max_iterations: 888,
                provider: "openrouter".into(),
                model: "review-model".into(),
                ..Default::default()
            },
        ),
    ]);

    let merged = super::merged_agent_configs(&configured);

    let main = merged.get("main").expect("main config");
    assert_eq!(main.label.as_deref(), Some("主代理"));
    assert_eq!(main.max_parallel_subagents, 1);
    assert!(!main.allow_as_subagent);
    assert_eq!(main.max_depth, 0);
    assert!(!main.hidden);
    assert_eq!(main.permission["*"], "deny");
    assert_eq!(main.max_iterations, 777);
    assert_eq!(main.provider, "openrouter");
    assert_eq!(main.model, "main-model");
    assert_eq!(main.system_prompt.as_deref(), Some("覆盖提示词"));

    let reviewer = merged.get("reviewer").expect("reviewer config");
    assert!(reviewer.allow_as_subagent);
    assert_eq!(reviewer.max_depth, 1);
    assert_eq!(reviewer.permission["*"], "deny");
    assert_eq!(reviewer.max_iterations, 888);
    assert_eq!(reviewer.allowed_tools, vec!["file_read"]);
    assert_eq!(reviewer.provider, "openrouter");
    assert_eq!(reviewer.model, "review-model");
}
