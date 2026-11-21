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
fn deepagents_session_skills_use_a_separate_reviewed_install() {
    let contract = super::contract("deepagents").unwrap();
    assert!(
        super::installed_directory("deepagents", &contract)
            .unwrap()
            .ends_with("deepagents-0.1.42-model-source-v5")
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
    let mut contract = super::contract("harn").expect("Harn must have a reviewed runtime");
    assert_eq!(contract.layout, "standalone");
    assert_eq!(contract.version, "0.10.201");
    assert!(contract.files.contains_key("harn"));
    assert!(contract.files.contains_key("agent.harn"));
    assert!(contract.files.contains_key("shiju_stdio.py"));
    assert!(
        super::installed_directory("harn", &contract)
            .unwrap()
            .ends_with("harn-0.10.201-model-source-v5")
    );
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("harn"), "tampered").unwrap();
    assert!(super::verify(directory.path(), &contract).is_err());
    for (name, expected) in &mut contract.files {
        std::fs::write(directory.path().join(name), name.as_bytes()).unwrap();
        *expected = hex::encode(Sha256::digest(name.as_bytes()));
    }
    let native = directory.path().join("harn-native");
    std::fs::write(&native, b"engine").unwrap();
    let receipt = serde_json::json!({
        "version": contract.version,
        "source_sha256": contract.source_sha256,
        "patch_sha256": contract.patch_sha256,
        "engine_sha256": hex::encode(Sha256::digest(b"engine")),
    });
    std::fs::write(directory.path().join("build-receipt.json"), receipt.to_string()).unwrap();
    assert!(super::verify(directory.path(), &contract).is_ok());
    std::fs::write(&native, b"modified").unwrap();
    assert!(super::verify(directory.path(), &contract).unwrap_err().contains("checksum"));
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
    assert!(contract.source_sha256.is_some());
    let patch = include_str!("../../../../patches/acp-model-source/stakpak/skills.patch");
    assert_eq!(
        contract.patch_sha256.as_deref().unwrap(),
        hex::encode(Sha256::digest(patch.as_bytes())),
    );
    assert!(contract.files.contains_key("mcp.py"));
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

#[tokio::test]
async fn glm_managed_runtime_cold_install_initializes_and_rejects_redirection() {
    const AGENT: &str = "glm-acp-agent";
    let _lock = crate::TEST_HOME_ENV_LOCK.lock().await;
    struct TestEnv(&'static str, Option<std::ffi::OsString>);
    impl TestEnv {
        fn set(name: &'static str, value: impl AsRef<std::ffi::OsStr>) -> Self {
            let previous = std::env::var_os(name);
            unsafe { std::env::set_var(name, value) };
            Self(name, previous)
        }
    }
    impl Drop for TestEnv {
        fn drop(&mut self) {
            unsafe {
                match &self.1 {
                    Some(value) => std::env::set_var(self.0, value),
                    None => std::env::remove_var(self.0),
                }
            }
        }
    }
    let temp = tempfile::tempdir().unwrap();
    let config = temp.path().join("GLM 受管");
    let _root = TestEnv::set("VIBEWINDOW_CONFIG_DIR", &config);

    let contract = contract(AGENT).unwrap();
    let directory = installed_directory(AGENT, &contract).unwrap();
    let expected = directory.join("node_modules").join(AGENT).join(&contract.bin);
    assert!(!directory.exists());
    {
        // An empty offline cache guarantees the supplier install fails in the stage.
        let _cache = TestEnv::set("npm_config_cache", temp.path().join("offline npm cache"));
        let _offline = TestEnv::set("npm_config_offline", "true");
        let failure = crate::install_builtin_agent(AGENT).await.unwrap_err();
        assert!(failure.contains("安装失败"), "{failure}");
        assert!(!directory.exists());
        assert_eq!(std::fs::read_dir(root(AGENT).unwrap()).unwrap().count(), 0);
    }
    crate::install_builtin_agent(AGENT).await.unwrap();
    let registered = crate::resolve_agent_spec(AGENT).unwrap();
    assert_eq!(Path::new(&registered.command), expected);
    assert!(registered.args.is_empty());
    crate::install_builtin_agent(AGENT).await.unwrap();

    let node = shell_command::resolve_executable(if cfg!(windows) { "node.exe" } else { "node" })
        .unwrap();
    let mut child = tokio::process::Command::new(node)
        .arg(&registered.command)
        .current_dir(&config)
        .env("HOME", &config)
        .env("USERPROFILE", &config)
        .env("APPDATA", &config)
        .env("LOCALAPPDATA", &config)
        .env("XDG_CONFIG_HOME", &config)
        .env("XDG_STATE_HOME", &config)
        .env_remove("NODE_OPTIONS")
        .env_remove("NODE_PATH")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    // Keep the input pipe open through initialize and terminate the same Node child.
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    let initialized = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let mut input = child.stdin.take().unwrap();
        let mut output = BufReader::new(child.stdout.take().unwrap()).lines();
        let request = serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": {"protocolVersion": 1, "clientCapabilities": {}}
        });
        input.write_all(format!("{request}\n").as_bytes()).await.unwrap();
        while let Some(line) = output.next_line().await.unwrap() {
            if let Ok(message) = serde_json::from_str::<serde_json::Value>(&line)
                && message["id"] == 1
            {
                return message;
            }
        }
        panic!("GLM closed stdout before initialize");
    })
    .await
    .expect("GLM initialize timed out");
    assert_eq!(initialized["result"]["protocolVersion"], 1, "{initialized}");
    let _ = child.kill().await;
    child.wait().await.unwrap();

    let shim = directory.join(if cfg!(windows) {
        "node_modules/.bin/glm-acp-agent.cmd"
    } else {
        "node_modules/.bin/glm-acp-agent"
    });
    if shim.symlink_metadata().is_ok() {
        std::fs::remove_file(&shim).unwrap();
    }
    std::fs::write(&shim, "unreviewed npm shim").unwrap();
    assert!(verify_reviewed_agent_executable(AGENT, shim.to_str().unwrap()).is_err());
    assert_eq!(crate::resolve_agent_spec(AGENT).unwrap().command, registered.command);

    #[cfg(unix)]
    {
        let copy = config.join("redirected.js");
        std::fs::rename(&expected, &copy).unwrap();
        std::os::unix::fs::symlink(&copy, &expected).unwrap();
        assert!(reviewed_agent_command(AGENT).is_err());
        std::fs::remove_file(&expected).unwrap();
        std::fs::rename(copy, &expected).unwrap();
    }
    std::fs::write(&expected, "tampered").unwrap();
    assert!(verify_reviewed_agent_executable(AGENT, &registered.command).is_err());
    assert!(crate::install_builtin_agent(AGENT).await.is_err());
}
