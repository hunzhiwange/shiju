use crate::project;
use std::path::{Path, PathBuf};
use std::process::Command;
use config_worktree::WorktreeSettings;

fn git(path: &Path, args: &[&str]) -> String {
    let output = Command::new("git").arg("-C").arg(path).args(args).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8_lossy(&output.stdout).trim().into()
}
fn fixture() -> (tempfile::TempDir, PathBuf, project::Info) {
    let temp = tempfile::tempdir().unwrap();
    let primary = temp.path().join("primary");
    std::fs::create_dir(&primary).unwrap();
    git(&primary, &["init", "-b", "main"]);
    commit(&primary);
    let info = project::Info {
        id: "creation-policy".into(),
        worktree: primary.to_string_lossy().into(),
        vcs: Some(project::Vcs::Git),
        name: None,
        icon: None,
        commands: None,
        time: project::TimeInfo { created: 0, updated: 0, initialized: None },
        sandboxes: Vec::new(),
    };
    (temp, primary, info)
}
fn commit(path: &Path) {
    git(
        path,
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.com",
            "commit",
            "--allow-empty",
            "-m",
            "change",
        ],
    );
}

#[tokio::test]
async fn worktree_creation_policy_normal_and_fork_share_tenant_settings() {
    super::initialize_host();
    let (temp, primary, _) = fixture();
    let root = temp.path().join("selected");
    let tenant =
        vw_api_transport::TenantScope::new("creation-policy", uuid::Uuid::new_v4().to_string())
            .unwrap();
    crate::session::ui_store::with_tenant_scope(tenant.clone(), async {
        // Initialization reads the authoritative project settings in the active tenant.
        project::from_directory(&tenant, &primary).await.unwrap();
        crate::config::update_global_for_tenant(
            &tenant,
            serde_json::json!({"worktree_settings": {"root": root}}),
        )
        .await
        .unwrap();
        let expected = root.clone();
        project::instance::provide(&primary, None, move || {
            Box::pin(async move {
                let created = super::create(Some(super::CreateInput {
                    name: Some("ordinary".into()),
                    start_command: None,
                }))
                .await
                .unwrap();
                assert_eq!(
                    Path::new(&created.directory).parent().unwrap(),
                    std::fs::canonicalize(&expected).unwrap()
                );
                let fork = super::create_fork_checkout(Some("fork")).await.unwrap();
                assert_eq!(
                    Path::new(&fork.directory).parent().unwrap(),
                    std::fs::canonicalize(&expected).unwrap()
                );
                super::remove_fork_checkout(fork.directory).await.unwrap();
                super::remove_fork_checkout(created.directory).await.unwrap();
            })
        })
        .await
        .unwrap();
    })
    .await;
}
