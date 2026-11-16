use std::{path::Path, process::Command};
use gateway_agent_apps::mcp_acceptance::security::{ENV, A, PEER, B, ENV_SECRET, HEADER_SECRET, private_dir, run};

#[tokio::test]
async fn cc_mcp_t51_foreign_tenants_accounts_and_job_owners_cannot_cross_boundaries() {
    let Some(root) = std::env::var_os(ENV) else {
        let root = tempfile::tempdir().unwrap();
        private_dir(root.path());
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                concat!(
                    module_path!(),
                    "::cc_mcp_t51_foreign_tenants_accounts_and_job_owners_cannot_cross_boundaries"
                )
                .split_once("::")
                .unwrap()
                .1,
                "--nocapture",
            ])
            .env(ENV, root.path())
            .env("SHIJU_AGENT_APPS_ACCESS_FILE", root.path().join("access.json"))
            .output()
            .unwrap();
        let logs = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        for secret in [A, PEER, B, ENV_SECRET, HEADER_SECRET] {
            assert!(!logs.contains(secret), "process logs disclosed a planted token");
        }
        assert!(output.status.success(), "isolated security regression failed: {logs}");
        assert!(root.path().join("verified").exists(), "isolated test must execute assertions");
        return;
    };
    run(Path::new(&root), crate::gateway::dynamic_tools_acceptance_router).await;
}
