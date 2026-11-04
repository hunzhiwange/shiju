use super::load_agent_configs;
use serde_json::json;
use std::collections::HashSet;
use vw_config_types::agent::{AgentRuntime, BUILTIN_AGENT_SPECS};

#[test]
fn agent_catalog_covers_twenty_two_unique_backends() {
    let catalog = load_agent_configs(&json!({}));
    let backends = catalog
        .values()
        .filter_map(|entry| match &entry.runtime {
            AgentRuntime::Acp { backend } => Some(backend.as_str()),
            AgentRuntime::Native => None,
        })
        .collect::<HashSet<_>>();
    assert_eq!(backends.len(), 22);
    assert!(catalog["main"].builtin);
    assert!(!catalog["acp-snow"].enabled);
    assert_eq!(
        catalog
            .values()
            .filter(|entry| matches!(entry.runtime, AgentRuntime::Acp { .. }) && entry.enabled)
            .count(),
        6
    );
    for backend in vw_acp::built_in_agent_specs().keys() {
        let entry = &catalog[&format!("acp-{backend}")];
        assert!(entry.builtin);
        assert!(!entry.recommended_prompts.is_empty());
    }
}

#[test]
fn agent_catalog_preserves_native_role_limits() {
    let catalog = load_agent_configs(&json!({}));
    for spec in BUILTIN_AGENT_SPECS {
        let entry = &catalog[spec.key];
        assert_eq!(entry.mode, spec.mode);
        assert_eq!(entry.allowed_tools, spec.allowed_tools);
        assert_eq!(entry.allow_as_subagent, spec.allow_as_subagent);
        assert_eq!(entry.runtime, AgentRuntime::Native);
    }
}

#[test]
fn agent_catalog_preserves_official_identity_and_editable_overrides() {
    let config = json!({"agents": {"acp-codex": {
        "provider": "", "model": "", "builtin": false,
        "runtime": {"kind": "native"}, "enabled": false,
        "label": "My CLI", "description": "Edited", "system_prompt": "Rules",
        "recommended_prompts": ["Review"],
        "defaults": {"skills": {"mode": "fixed", "value": []}}
    }}});
    let catalog = load_agent_configs(&config);
    let entry = &catalog["acp-codex"];
    assert!(entry.builtin);
    assert_eq!(entry.runtime, AgentRuntime::Acp { backend: "codex".into() });
    assert!(!entry.enabled);
    assert_eq!(entry.label.as_deref(), Some("My CLI"));
    assert_eq!(entry.description.as_deref(), Some("Edited"));
    assert_eq!(
        serde_json::to_value(&entry.defaults).unwrap()["skills"],
        json!({"mode": "fixed", "value": []})
    );
    assert_eq!(entry.system_prompt.as_deref(), Some("Rules"));
    assert_eq!(entry.recommended_prompts, ["Review"]);
    let reloaded = load_agent_configs(&json!({"agents": catalog}));
    assert_eq!(
        serde_json::to_value(&reloaded["acp-codex"]).unwrap(),
        serde_json::to_value(entry).unwrap()
    );
}

#[test]
fn agent_catalog_keeps_all_backends_without_installed_clis() {
    const CHILD: &str = "SHIJU_CATALOG_EMPTY_CLI_ENV";
    if std::env::var_os(CHILD).is_some() {
        let catalog = load_agent_configs(&json!({}));
        assert_eq!(catalog.keys().filter(|key| key.starts_with("acp-")).count(), 22);
        assert_eq!(catalog["acp-snow"].runtime, AgentRuntime::Acp { backend: "snow".into() });
        return;
    }
    // Isolate executable lookup from this process and the developer's installed CLIs.
    let empty_home =
        std::env::temp_dir().join(format!("shiju-catalog-empty-{}", std::process::id()));
    std::fs::create_dir_all(&empty_home).unwrap();
    let result = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["agent_catalog_keeps_all_backends_without_installed_clis", "--nocapture"])
        .env_clear()
        .env(CHILD, "1")
        .env("PATH", &empty_home)
        .env("HOME", &empty_home)
        .env("USERPROFILE", &empty_home)
        .env("XDG_BIN_HOME", &empty_home)
        .output()
        .unwrap();
    std::fs::remove_dir_all(&empty_home).unwrap();
    assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stdout));
}
