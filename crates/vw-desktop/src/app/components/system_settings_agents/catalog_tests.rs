use super::catalog;
use crate::app::state::{AgentsSettingsState, DelegateAgentSettingsEntry};

#[test]
fn agent_catalog_view_shares_enabled_state_across_tabs() {
    let config: vw_config_types::config::Config = serde_json::from_value(serde_json::json!({})).unwrap();
    let mut settings = AgentsSettingsState::default();
    settings.entries = crate::app::state::agent_settings_entries_from_config(&config);
    assert_eq!(catalog::entries(&settings, true, "").iter().filter(|e| e.key.starts_with("acp-")).count(), 22);
    let key = "acp-codex";
    settings.entries.iter_mut().find(|e| e.key == key).unwrap().enabled = true;
    let enabled = catalog::entries(&settings, false, "codex");
    let official = catalog::entries(&settings, true, "codex");
    assert!(std::ptr::eq(enabled[0], official[0]));
    settings.entries.iter_mut().find(|e| e.key == key).unwrap().enabled = false;
    assert!(catalog::entries(&settings, false, "codex").is_empty());
    assert!(!catalog::entries(&settings, true, "codex")[0].enabled);
}

#[test]
fn agent_catalog_view_keeps_disabled_custom_entries_manageable() {
    let mut settings = AgentsSettingsState::default();
    let mut custom = DelegateAgentSettingsEntry::from_config("my-agent", None);
    custom.enabled = false;
    custom.label = "中文资料".into();
    settings.entries.push(custom);
    assert_eq!(catalog::entries(&settings, false, "中文资料")[0].key, "my-agent");
    assert!(catalog::entries(&settings, true, "中文资料").is_empty());
    assert!(catalog::entries(&settings, false, "missing").is_empty());
}
