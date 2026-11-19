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
    let _lock = crate::TEST_HOME_ENV_LOCK.lock().await;
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

#[tokio::test]
#[ignore = "requires npm/network; installs only into an explicit isolated directory"]
async fn acp_onboarding_deepagents_reviewed_install_and_discovery() {
    let directory = std::env::var("VW_ACP_INSTALL_ACCEPTANCE_ROOT").unwrap();
    assert!(Path::new(&directory).is_absolute());
    let _lock = crate::TEST_HOME_ENV_LOCK.lock().await;
    struct Restore(Option<std::ffi::OsString>);
    impl Drop for Restore {
        fn drop(&mut self) {
            unsafe {
                match &self.0 {
                    Some(value) => std::env::set_var("VIBEWINDOW_CONFIG_DIR", value),
                    None => std::env::remove_var("VIBEWINDOW_CONFIG_DIR"),
                }
            }
        }
    }
    let _restore = Restore(std::env::var_os("VIBEWINDOW_CONFIG_DIR"));
    unsafe {
        std::env::set_var("VIBEWINDOW_CONFIG_DIR", &directory);
    }
    crate::install_builtin_agent("deepagents").await.unwrap();
    let (command, args) = reviewed_agent_command("deepagents").unwrap().unwrap();
    assert!(command.starts_with(&directory));
    assert!(args.is_empty());
    let registered = crate::resolve_agent_spec("deepagents").unwrap();
    assert_eq!(registered.command, command);
    assert_eq!(registered.args, args);
    verify_reviewed_agent_executable("deepagents", &command).unwrap();
    assert!(verify_reviewed_agent_executable("deepagents", "npx").is_err());
    println!("reviewed DeepAgents: {command}");
}

#[test]
fn deepagents_session_mcp_uses_a_separate_reviewed_install() {
    let contract = super::contract("deepagents").unwrap();
    assert!(
        super::installed_directory("deepagents", &contract)
            .unwrap()
            .ends_with("deepagents-0.1.42-model-source-v4")
    );
}

#[test]
fn droid_managed_patch_is_pinned_in_a_separate_install() {
    let contract = super::contract("droid").unwrap();
    assert_eq!(contract.layout, "standalone");
    assert_eq!(contract.version, "1.9.0");
    let original = contract.source_sha256.as_deref().unwrap();
    let patched = contract.files.get("droid").unwrap();
    assert_eq!(patched.len(), 64);
    assert_ne!(original, patched);
    assert!(
        super::installed_directory("droid", &contract)
            .unwrap()
            .ends_with("droid-1.9.0-model-source-v2")
    );
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("droid"), "tampered").unwrap();
    assert!(super::verify(directory.path(), &contract).unwrap_err().contains("checksum"));
}

#[test]
fn harn_reviewed_natural_language_runtime_is_pinned() {
    let contract = super::contract("harn").expect("Harn must have a reviewed runtime");
    assert_eq!(contract.layout, "standalone");
    assert_eq!(contract.version, "0.10.201");
    assert!(contract.files.contains_key("harn"));
    assert!(contract.files.contains_key("agent.harn"));
    assert!(contract.files.contains_key("shiju_stdio.py"));
    assert!(
        super::installed_directory("harn", &contract)
            .unwrap()
            .ends_with("harn-0.10.201-model-source-v3")
    );
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("harn"), "tampered").unwrap();
    assert!(super::verify(directory.path(), &contract).is_err());
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[tokio::test]
#[ignore = "installs the official Cortex Code release into an isolated directory"]
async fn cortex_code_managed_install_resolves_native_entry() {
    let root = std::env::var("VW_ACP_INSTALL_ACCEPTANCE_ROOT").unwrap();
    assert!(Path::new(&root).is_absolute());
    let _lock = crate::TEST_HOME_ENV_LOCK.lock().await;
    assert_eq!(std::env::var("VIBEWINDOW_CONFIG_DIR").as_deref(), Ok(root.as_str()));

    crate::install_builtin_agent("cortex-code").await.unwrap();
    let registered = crate::resolve_agent_spec("cortex-code").unwrap();
    assert!(Path::new(&registered.command).starts_with(root));
    assert_eq!(registered.args, ["acp", "serve"]);
    verify_reviewed_agent_executable("cortex-code", &registered.command).unwrap();
    let version =
        std::process::Command::new(registered.command).arg("--version").output().unwrap();
    assert!(version.status.success());
    assert_eq!(String::from_utf8(version.stdout).unwrap().trim(), "Cortex Code v1.2.46");
    crate::install_builtin_agent("cortex-code").await.unwrap();
}

#[test]
fn stakpak_reviewed_runtime_pins_native_and_launcher() {
    let contract = super::contract("stakpak").expect("Stakpak must have a reviewed runtime");
    assert_eq!(contract.version, "0.3.88");
    assert_eq!(contract.bin, "stakpak");
    assert!(contract.files.contains_key("stakpak-native"));
    assert!(contract.files.contains_key("stakpak"));
}

#[test]
fn mimo_code_runtime_requires_the_pinned_acp_patch_receipt() {
    let contract = contract("mimo-code").unwrap();
    let root = tempfile::tempdir().unwrap();
    let executable = root.path().join("mimo");
    std::fs::write(&executable, b"fixture executable").unwrap();
    let patch = include_str!("../../../../patches/acp-model-source/mimo-code/stdio-only.patch");
    assert_eq!(
        contract.patch_sha256.as_deref().unwrap(),
        hex::encode(Sha256::digest(patch.as_bytes())),
    );
    assert!(verify(root.path(), &contract).is_err());

    let receipt = serde_json::json!({
        "version": "0.1.17",
        "source_sha256": contract.source_sha256,
        "patch_sha256": contract.patch_sha256,
        "sha256": hex::encode(Sha256::digest(b"fixture executable")),
    });
    std::fs::write(root.path().join("build-receipt.json"), receipt.to_string()).unwrap();
    assert_eq!(verify(root.path(), &contract).unwrap(), executable);
    std::fs::write(executable, b"altered").unwrap();
    assert!(verify(root.path(), &contract).is_err());
}

#[test]
fn omp_uses_fixed_native_release() {
    let contract = super::contract("omp").expect("OMP must have a reviewed runtime");
    assert_eq!(contract.layout, "standalone");
    assert_eq!(contract.version, "18.22.0");
    assert_eq!(contract.bin, "omp");
    assert_eq!(
        contract.files["omp"],
        "d185113d63eba23925e5d70ab661a424b827c06c98f8e766c3bf4ad62ddaff9a"
    );
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[tokio::test]
#[ignore = "downloads the official Devin archive into an explicit isolated directory"]
async fn devin_managed_install_resolves_and_verifies_native_entry() {
    let root = PathBuf::from(std::env::var_os("VW_ACP_INSTALL_ACCEPTANCE_ROOT").unwrap());
    assert!(root.is_absolute());
    let _lock = crate::TEST_HOME_ENV_LOCK.lock().await;
    assert_eq!(std::env::var_os("VIBEWINDOW_CONFIG_DIR").as_deref(), Some(root.as_os_str()));
    assert!(reviewed_agent_command("devin").is_err());

    crate::install_builtin_agent("devin").await.unwrap();
    let registered = crate::resolve_agent_spec("devin").unwrap();
    assert_eq!(
        Path::new(&registered.command),
        root.join("acp-runtime/devin-3000.13.4-model-source-v1/devin")
    );
    assert_eq!(registered.args, ["acp", "--process-model", "in-process"]);
    verify_reviewed_agent_executable("devin", &registered.command).unwrap();
    let output = std::process::Command::new(&registered.command).arg("--version").output().unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8(output.stdout).unwrap().contains("3000.13.4"));
    crate::install_builtin_agent("devin").await.unwrap();
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[tokio::test]
#[ignore = "downloads the official Corust archive into an explicit isolated directory"]
async fn corust_managed_install_resolves_and_verifies_native_entry() {
    let root = std::env::var("VW_ACP_INSTALL_ACCEPTANCE_ROOT").unwrap();
    assert!(Path::new(&root).is_absolute());
    let _lock = crate::TEST_HOME_ENV_LOCK.lock().await;
    // The isolated test process receives VIBEWINDOW_CONFIG_DIR from its caller.
    assert_eq!(std::env::var("VIBEWINDOW_CONFIG_DIR").as_deref(), Ok(root.as_str()));
    crate::install_builtin_agent("corust-agent").await.unwrap();
    let registered = crate::resolve_agent_spec("corust-agent").unwrap();
    assert!(Path::new(&registered.command).starts_with(&root));
    assert!(registered.command.ends_with("/corust-agent-acp"));
    assert!(registered.args.is_empty());
    verify_reviewed_agent_executable("corust-agent", &registered.command).unwrap();
    crate::install_builtin_agent("corust-agent").await.unwrap();
}
