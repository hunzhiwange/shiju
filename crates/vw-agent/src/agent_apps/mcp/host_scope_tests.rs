use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Command};
use gateway_agent_apps::mcp_acceptance::host_scope::{ROOT, TEST, run};

fn directory(path: &Path) {
    fs::create_dir_all(path).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
}

#[tokio::test]
async fn cc_mcp_t52_unavailable_hosts_never_fall_back_and_local_users_remain_isolated() {
    let Some(root) = std::env::var_os(ROOT) else {
        let root = tempfile::tempdir().unwrap();
        directory(&root.path().join("desktop/.codex"));
        fs::write(root.path().join("desktop/.codex/config.toml"), b"# desktop sentinel\n").unwrap();
        let output = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", &format!("agent_apps::mcp::host_scope_tests::{TEST}"), "--nocapture"])
            .env(ROOT, root.path())
            .env("HOME", root.path().join("desktop"))
            .env("SHIJU_AGENT_APPS_ACCESS_FILE", root.path().join("access.json"))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "host regression failed: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(root.path().join("verified").is_file(), "child must execute behavior assertions");
        return;
    };
    run(Path::new(&root), crate::gateway::dynamic_tools_acceptance_router).await;
}
