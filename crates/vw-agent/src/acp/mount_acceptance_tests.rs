//! Invoke the production CLI over stdio, with real filesystem and MCP transports.
#[test]
fn acp_server_mount_acceptance_production_stdio_tools_update_and_restart() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .expect("acceptance requires an absolute shared-slot CARGO_TARGET_DIR");
    let binary = std::path::PathBuf::from(target).join("debug/shiju");
    assert!(binary.is_absolute() && binary.is_file(), "build vw-cli shiju in this slot first");
    let output = tempfile::tempdir().unwrap();
    let result = std::process::Command::new("python3")
        .arg(root.join("scripts/acp_mount_integration.py"))
        .arg("--shiju")
        .arg(binary)
        .arg("--output")
        .arg(output.path().join("evidence"))
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "production mount acceptance failed: {} {}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(
        &std::fs::read(output.path().join("evidence/mount-results.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(report["passed"], true);
    assert!(report["production_starts"].as_u64().unwrap() >= 2);
    assert!(report["checks"].as_object().unwrap().len() >= 12);
}

#[test]
fn acp_server_mount_acceptance_tenant_and_same_directory_session_isolation() {
    let root = tempfile::tempdir().unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "acp::mount_acceptance_tests::isolation_worker", "--nocapture"])
        .env("ACP_ACCEPTANCE_ISOLATION_ROOT", root.path())
        .env("VW_MODEL_RECOVERY_TEST_ROOT", root.path().join("home"))
        .env("VW_ACP_MOUNT_RECOVERY_DB_ROOT", root.path().join("home"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[tokio::test]
async fn isolation_worker() {
    use super::VibeWindowBackend;
    use vw_acp::server::{BackendMountRequest, SessionBackend};
    use vw_acp::{SessionMountSelection, SessionSkillMount};
    let Some(root) = std::env::var_os("ACP_ACCEPTANCE_ISOLATION_ROOT") else { return };
    let root = std::path::PathBuf::from(root);
    let source = root.join("source");
    std::fs::create_dir_all(root.join("home")).unwrap();
    std::fs::create_dir(&source).unwrap();
    std::fs::write(
        source.join("SKILL.md"),
        "---\nname: isolated_probe\ndescription: isolated source\n---\nScoped body\n",
    )
    .unwrap();
    let backend = VibeWindowBackend::default();
    let first = backend
        .create_session_with_mounts(
            root.clone(),
            BackendMountRequest {
                skills: Some(SessionMountSelection {
                    version: 1,
                    skills: vec![SessionSkillMount {
                        name: "isolated_probe".into(),
                        path: source.to_str().unwrap().into(),
                    }],
                }),
                mcp_servers: vec![],
            },
        )
        .await
        .unwrap();
    let empty = || BackendMountRequest { skills: None, mcp_servers: vec![] };
    let second = backend.create_session_with_mounts(root.clone(), empty()).await.unwrap();
    assert_ne!(first.session.id, second.session.id);
    assert!(second.mounts.skills.is_empty());
    assert!(backend.mounted_runtime(&second.session.id).0.mounts().is_empty());
    assert_eq!(backend.mounted_runtime(&first.session.id).0.mounts().len(), 1);
    let foreign = VibeWindowBackend::new(
        vw_api_types::id::TenantScope::new("other-platform", "other-company").unwrap(),
    );
    assert!(
        foreign.load_session_with_mounts(&first.session.id, root.clone(), empty()).await.is_err()
    );
    let own = backend.load_session_with_mounts(&first.session.id, root, empty()).await.unwrap();
    assert_eq!(own.mounts.skills, ["isolated_probe"]);
}
