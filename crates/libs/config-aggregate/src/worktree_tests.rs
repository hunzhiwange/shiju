use crate::Config;
use config_worktree::WorktreeSettings;

#[test]
fn config_preserves_worktree_settings_defaults_debug_and_schema() {
    let settings = WorktreeSettings::default();
    let config: Config = serde_json::from_str("{}").unwrap();
    assert_eq!(config.worktree_settings, settings);
    assert!(format!("{config:?}").contains("worktree_settings"));
    let schema = serde_json::to_value(schemars::schema_for!(Config)).unwrap();
    assert!(schema["properties"].get("worktree_settings").is_some());
}
