use crate::state::DelegateAgentSettingsEntry;
fn catalog_config() -> config_aggregate::Config {
    // Wire-shaped catalog fixture: official identities come from the gateway, not CLI detection.
    let mut config: config_aggregate::Config =
        serde_json::from_value(serde_json::json!({})).unwrap();
    for backend in [
        "auggie",
        "claude",
        "codex",
        "codebuddy",
        "goose",
        "vibe",
        "hermes",
        "snow",
        "copilot",
        "cursor",
        "droid",
        "gemini",
        "iflow",
        "kilocode",
        "kimi",
        "kiro",
        "opencode",
        "openclaw",
        "pi",
        "qoder",
        "qwen",
        "trae",
    ] {
        let mut value =
            serde_json::to_value(config_agent::DelegateAgentConfig::default()).unwrap();
        let fields = serde_json::json!({"builtin":true, "enabled": backend == "codex", "label":backend,
            "runtime":{"kind":"acp","backend":backend}});
        value.as_object_mut().unwrap().extend(fields.as_object().unwrap().clone());
        config.agents.insert(format!("acp-{backend}"), serde_json::from_value(value).unwrap());
    }
    config
}

#[test]
fn agent_catalog_view_shares_enabled_state_across_tabs() {
    let config = catalog_config();
    let mut entries = crate::state::agent_settings_entries_from_config(&config);
    assert_eq!(
        super::entries(&entries, true, "").iter().filter(|e| e.key.starts_with("acp-")).count(),
        22
    );
    let key = "acp-codex";
    entries.iter_mut().find(|e| e.key == key).unwrap().enabled = true;
    let enabled = super::entries(&entries, false, "codex");
    let official = super::entries(&entries, true, "codex");
    assert!(std::ptr::eq(enabled[0], official[0]));
    entries.iter_mut().find(|e| e.key == key).unwrap().enabled = false;
    assert!(super::entries(&entries, false, "codex").is_empty());
    assert!(!super::entries(&entries, true, "codex")[0].enabled);
}

#[test]
fn agent_catalog_view_keeps_disabled_custom_entries_manageable() {
    let mut entries = config_agent::builtin_agent_keys().into_iter().map(|key| DelegateAgentSettingsEntry::from_config(key, None)).collect::<Vec<_>>();
    let mut custom = DelegateAgentSettingsEntry::from_config("my-agent", None);
    custom.enabled = false;
    custom.label = "中文资料".into();
    entries.push(custom);
    assert_eq!(super::entries(&entries, false, "中文资料")[0].key, "my-agent");
    assert!(super::entries(&entries, true, "中文资料").is_empty());
    assert!(super::entries(&entries, false, "missing").is_empty());
}
