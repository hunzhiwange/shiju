use super::*;

#[test]
fn acp_edit_preserves_enabled_state() {
    let old = AcpAgentConfig { command: "old".into(), ..Default::default() };
    let catalog = HashMap::from([("codex".into(), old.clone())]);
    let configured = HashMap::from([("other".into(), old)]);
    let edited = AcpAgentConfig {
        command: "new".into(),
        args: vec!["a b".into()],
        env: HashMap::from([("KEY".into(), "value".into())]),
    };
    let patch = build_acp_agent_edit_patch("codex", edited, &catalog, configured).unwrap();
    assert!(patch.get("acp").is_none());
    assert_eq!(patch["acp_saved"]["codex"]["command"], "new");
    assert_eq!(patch["acp_saved"]["codex"]["args"][0], "a b");
    assert_eq!(patch["acp_saved"]["codex"]["env"]["KEY"], "value");
}

#[test]
fn acp_edit_preserves_enabled_state_enabled_and_removes_old_env() {
    let old = AcpAgentConfig {
        command: "old".into(),
        env: HashMap::from([("OLD".into(), "secret".into())]),
        ..Default::default()
    };
    let catalog = HashMap::from([("codex".into(), old.clone())]);
    let configured = HashMap::from([("codex".into(), old.clone()), ("other".into(), old)]);
    let edited = AcpAgentConfig { command: "new".into(), ..Default::default() };
    let patch = build_acp_agent_edit_patch("codex", edited, &catalog, configured).unwrap();
    assert_eq!(patch["acp"]["codex"]["command"], "new");
    assert!(patch["acp"]["codex"]["env"]["OLD"].is_null());
    assert!(patch["acp"].get("other").is_none());
}

#[test]
fn acp_edit_preserves_enabled_state_saved_config_round_trip() {
    let spec = AcpAgentConfig {
        command: "driver".into(),
        args: vec!["two words".into(), "".into()],
        env: HashMap::from([("KEY".into(), "value".into())]),
    };
    let mut config = Config::default();
    config.acp.insert("other".into(), AcpAgentConfig::default());
    config.acp_saved.insert("codex".into(), spec.clone());
    let file = tempfile::NamedTempFile::new().unwrap();
    serde_json::to_writer(file.as_file(), &config).unwrap();
    let loaded: Config =
        serde_json::from_reader(std::fs::File::open(file.path()).unwrap()).unwrap();
    assert!(!loaded.acp.contains_key("codex"));
    assert!(loaded.acp.contains_key("other"));
    let saved = &loaded.acp_saved["codex"];
    assert_eq!(saved.command, spec.command);
    assert_eq!(saved.args, spec.args);
    assert_eq!(saved.env, spec.env);
}

#[test]
fn acp_configuration_flow_regression_new_agent_preserves_argument_boundaries() {
    let spec = AcpAgentConfig {
        command: "python3".into(),
        args: vec![" leading and trailing ".into(), "".into(), "中文,参数".into(), "  ".into()],
        env: HashMap::from([("FIXTURE_VALUE".into(), " 中文值 ".into())]),
    };
    let patch =
        build_acp_agent_create_patch("fixture", spec.clone(), &HashMap::new(), HashMap::new())
            .expect("new agent patch");
    let file = tempfile::NamedTempFile::new().unwrap();
    serde_json::to_writer(file.as_file(), &patch).unwrap();
    let saved: serde_json::Value =
        serde_json::from_reader(std::fs::File::open(file.path()).unwrap()).unwrap();
    let loaded: AcpAgentConfig = serde_json::from_value(saved["acp"]["fixture"].clone()).unwrap();
    assert_eq!(
        loaded.args, spec.args,
        "adding and editing must preserve the same argument boundaries"
    );
    assert_eq!(loaded.env, spec.env);
}
