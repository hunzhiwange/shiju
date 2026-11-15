use crate::{config::Config, worktree::WorktreeSettings};

#[test]
fn worktree_settings_defaults_are_safe() {
    let settings: WorktreeSettings = serde_json::from_str("{}").unwrap();
    assert_eq!(settings.keep_count, 15);
    assert!(!settings.fetch_before_create);
    assert!(settings.auto_cleanup_enabled);
    assert!(settings.ask_before_delete);
    assert!(settings.root.is_empty());
    assert_eq!(settings, WorktreeSettings::default());
    let config: Config = serde_json::from_str("{}").unwrap();
    assert_eq!(config.worktree_settings, settings);
    assert!(format!("{config:?}").contains("worktree_settings"));
    let schema = serde_json::to_value(schemars::schema_for!(Config)).unwrap();
    assert!(schema["properties"].get("worktree_settings").is_some());
}

#[test]
fn worktree_settings_rejects_invalid_values() {
    let home = std::env::temp_dir();
    for root in ["", "  ", "\t\n"] {
        let settings = WorktreeSettings { root: root.into(), ..Default::default() };
        assert!(settings.validate().is_ok());
        assert_eq!(settings.resolve_root(&home).unwrap(), crate::paths::task_worktrees_dir(&home));
    }
    let absolute = home.join("managed trees");
    let settings = WorktreeSettings {
        root: absolute.to_str().unwrap().into(),
        keep_count: u32::MAX,
        ..Default::default()
    };
    assert!(settings.validate().is_ok());
    assert_eq!(settings.resolve_root(&home).unwrap(), absolute);
    for root in ["relative", "../trees", "~/trees"] {
        let settings = WorktreeSettings { root: root.into(), ..Default::default() };
        assert!(settings.validate().is_err(), "relative root: {root}");
        assert!(settings.resolve_root(&home).is_err());
    }
    assert!(WorktreeSettings { keep_count: 0, ..Default::default() }.validate().is_err());
    for raw in ["-1", "4294967296", "1.5", "\"15\""] {
        assert!(
            serde_json::from_str::<WorktreeSettings>(&format!(r#"{{"keep_count":{raw}}}"#))
                .is_err(),
            "{raw}"
        );
    }
    let settings: WorktreeSettings = serde_json::from_str(r#"{"keep_count":1,"fetch_before_create":true,"auto_cleanup_enabled":false,"ask_before_delete":false}"#).unwrap();
    assert!(settings.validate().is_ok());
    assert_eq!(
        serde_json::from_value::<WorktreeSettings>(serde_json::to_value(&settings).unwrap())
            .unwrap(),
        settings
    );
}
