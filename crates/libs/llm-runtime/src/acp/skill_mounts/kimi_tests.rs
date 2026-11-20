use super::{append_skill_root, configure, verify_entry};
use agent_config::schema::AcpAgentConfig;

#[cfg(unix)]
#[test]
fn acp_skill_coexistence_kimi_appends_one_root_and_preserves_settings() {
    use std::{fs, os::unix::fs::PermissionsExt};

    let home = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    fs::set_permissions(home.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let config_path = home.path().join("config.toml");
    fs::write(
        &config_path,
        "builtin_product_skills = false\n[models.session-selected]\nmax_context_size = 100000\n",
    )
    .unwrap();

    append_skill_root(home.path(), root.path()).unwrap();
    let content = fs::read_to_string(&config_path).unwrap();
    let settings = content.parse::<toml::Table>().unwrap();
    assert_eq!(settings["builtin_product_skills"].as_bool(), Some(false));
    assert_eq!(
        settings["models"]["session-selected"]["max_context_size"].as_integer(),
        Some(100000)
    );
    assert_eq!(
        settings["extra_skill_dirs"].as_array().unwrap(),
        &[toml::Value::String(root.path().to_str().unwrap().into())]
    );
    assert!(append_skill_root(home.path(), home.path()).is_err());
    assert_eq!(fs::read_to_string(config_path).unwrap(), content);
}

#[test]
fn acp_skill_coexistence_kimi_preserves_unmounted_launch_and_rejects_unreviewed_entry() {
    let mut config = AcpAgentConfig {
        command: "local-kimi".into(),
        args: vec!["acp".into()],
        ..Default::default()
    };
    configure(&mut config, None).unwrap();
    assert_eq!(config.command, "local-kimi");
    assert_eq!(config.args, ["acp"]);
    assert!(config.env.is_empty());
    assert!(verify_entry(&config.command, &config.args).is_err());

    let unreviewed = tempfile::NamedTempFile::new().unwrap();
    assert!(verify_entry(unreviewed.path().to_str().unwrap(), &config.args).is_err());
}
