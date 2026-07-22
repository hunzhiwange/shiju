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
    assert_eq!(definition.max_depth, 3);
    assert_eq!(definition.max_iterations, 10);

    assert_eq!(super::BUILTIN_AGENT_SPECS.len(), 11);
    assert_eq!(super::BUILTIN_AGENT_SPECS[0].key, "main");
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
    let build = super::builtin_agent_config("build").expect("build preset");
    assert_eq!(build.mode, "primary");
    assert_eq!(build.permission["question"], "allow");

    let plan = super::builtin_agent_config("plan").expect("plan preset");
    assert_eq!(plan.permission["edit"]["*"], "deny");
    assert_eq!(plan.permission["edit"][".vibewindow/plans/*.md"], "allow");

    let explore = super::builtin_agent_config("explore").expect("explore preset");
    assert_eq!(explore.mode, "subagent");
    assert_eq!(explore.permission["*"], "deny");
    assert_eq!(explore.permission["read"], "allow");

    let summary = super::builtin_agent_config("summary").expect("summary preset");
    assert!(summary.hidden);
    assert_eq!(summary.permission["*"], "deny");

    assert!(super::builtin_agent_config("unknown").is_none());
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
    assert_eq!(coder.label.as_deref(), Some("Coder"));
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

    assert_eq!(
        keys,
        vec![
            "main",
            "researcher",
            "coder",
            "reviewer",
            "build",
            "plan",
            "general",
            "explore",
            "compaction",
            "title",
            "summary",
        ]
    );
}

#[test]
fn merged_builtin_overrides_preserve_explicit_metadata_and_mark_builtin() {
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

    assert_eq!(reviewer.label.as_deref(), Some("Strict Reviewer"));
    assert_eq!(reviewer.description.as_deref(), Some("custom review role"));
    assert!(reviewer.builtin);
}
