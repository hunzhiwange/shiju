use crate::{computer_control::*, config::Config};

#[test]
fn cc_control_03_config_save_reload_preserves_user_grants_and_version() {
    let mut config = Config::default();
    let mut user =
        ComputerControlUserConfig { native_apps: true, chrome: true, ..Default::default() };
    user.applications.insert("com.apple.TextEdit".into(), GrantDecision::Allow);
    user.applications.insert("com.apple.Preview".into(), GrantDecision::Deny);
    user.chrome_origins.insert("https://example.com".into(), GrantDecision::Allow);
    user.excel_documents.insert("book-1".into(), GrantDecision::Allow);
    config.computer_control.users.insert("uid:501".into(), user);
    config.computer_control.permission_version = 7;
    let bytes = serde_json::to_vec(&config).unwrap();
    let path = std::env::temp_dir().join(format!("shiju-cc03-{}.json", std::process::id()));
    // create_new prevents following an existing path or overwriting another user's file.
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&path).unwrap();
    std::io::Write::write_all(&mut file, &bytes).unwrap();
    drop(file);
    let reloaded: Config = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    std::fs::remove_file(path).unwrap();
    assert_eq!(reloaded.computer_control, config.computer_control);
    assert!(!reloaded.computer_control.users.contains_key("uid:502"));
    assert!(!reloaded.computer_control.users["uid:501"].locked_use);
}

#[test]
fn cc_control_03_legacy_config_denies_and_sensitive_fields_are_rejected() {
    let config: Config = serde_json::from_str("{}").unwrap();
    assert_eq!(config.computer_control, ComputerControlConfig::default());
    let user = ComputerControlUserConfig::default();
    assert!(!user.native_apps && !user.chrome && !user.excel && !user.word && !user.locked_use);
    for key in ["pairing_token", "document_body", "screenshot"] {
        let value = serde_json::json!({key: "sensitive"});
        assert!(serde_json::from_value::<ComputerControlConfig>(value.clone()).is_err());
        assert!(serde_json::from_value::<ComputerControlUserConfig>(value).is_err());
    }
}
