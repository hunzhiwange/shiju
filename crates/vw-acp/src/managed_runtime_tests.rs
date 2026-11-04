use super::*;

#[test]
fn acp_model_source_session_controls_artifact_tampering_is_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let contract = contract("codex").unwrap();
    let package = temp.path().join("node_modules").join(&contract.package);
    std::fs::create_dir_all(package.join("dist")).unwrap();
    std::fs::write(
        package.join("package.json"),
        r#"{"name":"@agentclientprotocol/codex-acp","version":"2.5.0"}"#,
    )
    .unwrap();
    std::fs::write(package.join("dist/index.js"), "altered").unwrap();
    assert!(verify(temp.path(), &contract).unwrap_err().contains("checksum"));
    assert!(super::contract("unknown-custom").is_none());
}

#[tokio::test]
#[ignore = "explicit real supplier installation; requires Node/npm/git/patch and network"]
async fn acp_model_source_session_controls_real_reviewed_installer_and_registry() {
    let root = std::env::var("VW_ACP_INSTALL_ACCEPTANCE_ROOT")
        .expect("explicit isolated installation root required");
    assert!(Path::new(&root).is_absolute());
    let _lock = crate::queue_paths::TEST_HOME_ENV_LOCK.lock().await;
    let original = std::env::var_os("VIBEWINDOW_CONFIG_DIR");
    unsafe {
        std::env::set_var("VIBEWINDOW_CONFIG_DIR", &root);
    }
    for agent in ["codex", "gemini", "iflow", "qwen", "pi", "vibe", "hermes", "goose"] {
        crate::install_builtin_agent(agent)
            .await
            .unwrap_or_else(|error| panic!("{agent}: {error}"));
        let (command, args) = reviewed_agent_command(agent).unwrap().unwrap();
        assert!(command.starts_with(&root));
        let registered = crate::resolve_agent_spec(agent).unwrap();
        assert_eq!(registered.command, command);
        assert_eq!(registered.args, args);
        println!("reviewed installer and registry verified: {agent}");
    }
    unsafe {
        match original {
            Some(value) => std::env::set_var("VIBEWINDOW_CONFIG_DIR", value),
            None => std::env::remove_var("VIBEWINDOW_CONFIG_DIR"),
        }
    }
}
