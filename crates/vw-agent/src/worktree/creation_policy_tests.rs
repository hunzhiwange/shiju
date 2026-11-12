use super::creation_policy::prepare;
use crate::project;
use std::path::{Path, PathBuf};
use std::process::Command;
use vw_config_types::worktree::WorktreeSettings;

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
async fn worktree_creation_policy_uses_configured_root() {
    let (temp, primary, info) = fixture();
    let root = temp.path().join("custom/nested");
    let settings = WorktreeSettings { root: root.to_string_lossy().into(), ..Default::default() };
    let actual = prepare(&info, primary.to_str().unwrap(), &settings).await.unwrap();
    assert_eq!(actual, std::fs::canonicalize(root).unwrap());
    assert!(actual.is_dir());
}
#[tokio::test]
async fn worktree_creation_policy_fetches_only_when_enabled() {
    let (temp, primary, info) = fixture();
    let remote = temp.path().join("remote.git");
    git(temp.path(), &["init", "--bare", remote.to_str().unwrap()]);
    git(&primary, &["remote", "add", "chosen", remote.to_str().unwrap()]);
    git(&primary, &["push", "-u", "chosen", "main"]);
    let head = git(&primary, &["rev-parse", "HEAD"]);
    let writer = temp.path().join("writer");
    git(temp.path(), &["clone", "-b", "main", remote.to_str().unwrap(), writer.to_str().unwrap()]);
    commit(&writer);
    git(&writer, &["push"]);
    let advanced = git(&writer, &["rev-parse", "HEAD"]);
    assert_ne!(head, advanced);
    let mut settings = WorktreeSettings {
        root: temp.path().join("trees").to_string_lossy().into(),
        ..Default::default()
    };
    prepare(&info, primary.to_str().unwrap(), &settings).await.unwrap();
    assert_eq!(git(&primary, &["rev-parse", "chosen/main"]), head);
    settings.fetch_before_create = true;
    let root = prepare(&info, primary.to_str().unwrap(), &settings).await.unwrap();
    assert_eq!(git(&primary, &["rev-parse", "chosen/main"]), advanced);
    assert_eq!(git(&primary, &["rev-parse", "HEAD"]), head);
    let checkout = root.join("fork");
    git(&primary, &["worktree", "add", "-b", "shiju/policy", checkout.to_str().unwrap()]);
    assert_eq!(git(&checkout, &["rev-parse", "HEAD"]), head);
}
#[tokio::test]
async fn worktree_creation_policy_fetch_failure_does_not_create_checkout() {
    let (temp, primary, info) = fixture();
    let root = temp.path().join("trees");
    let settings = WorktreeSettings {
        root: root.to_string_lossy().into(),
        fetch_before_create: true,
        ..Default::default()
    };
    let before = git(&primary, &["branch", "--list"]);
    let error = prepare(&info, primary.to_str().unwrap(), &settings).await.unwrap_err();
    assert!(error.to_string().contains("upstream"));
    git(&primary, &["remote", "add", "gone", temp.path().join("missing.git").to_str().unwrap()]);
    git(&primary, &["config", "branch.main.remote", "gone"]);
    git(&primary, &["config", "branch.main.merge", "refs/heads/main"]);
    assert!(prepare(&info, primary.to_str().unwrap(), &settings).await.is_err());
    assert_eq!(git(&primary, &["branch", "--list"]), before);
    assert_eq!(git(&primary, &["worktree", "list", "--porcelain"]).matches("worktree ").count(), 1);
    assert!(!root.exists());
}

#[cfg(unix)]
#[tokio::test]
async fn worktree_creation_policy_rejects_symlinks() {
    let (temp, primary, info) = fixture();
    let outside = temp.path().join("outside");
    std::fs::create_dir(&outside).unwrap();
    let root = temp.path().join("trees");
    std::os::unix::fs::symlink(&outside, &root).unwrap();
    let settings = WorktreeSettings { root: root.to_string_lossy().into(), ..Default::default() };
    assert!(prepare(&info, primary.to_str().unwrap(), &settings).await.is_err());
    std::fs::remove_file(&root).unwrap();
    let root = prepare(&info, primary.to_str().unwrap(), &settings).await.unwrap();
    let checkout = root.join("checkout");
    std::os::unix::fs::symlink(outside.join("missing"), &checkout).unwrap();
    assert!(super::creation_policy::validate_parent(&root, &checkout).await.is_err());
    std::fs::remove_file(&checkout).unwrap();
    let moved = temp.path().join("moved");
    std::fs::rename(&root, &moved).unwrap();
    std::os::unix::fs::symlink(&outside, &root).unwrap();
    assert!(super::creation_policy::validate_parent(&root, &checkout).await.is_err());
    assert!(std::fs::read_dir(&outside).unwrap().next().is_none());
}

#[tokio::test]
async fn worktree_creation_policy_normal_and_fork_share_tenant_settings() {
    let (temp, primary, _) = fixture();
    let root = temp.path().join("selected");
    let tenant =
        vw_api_types::id::TenantScope::new("creation-policy", uuid::Uuid::new_v4().to_string())
            .unwrap();
    crate::session::ui_store::with_tenant_scope(tenant.clone(), async {
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
                // The existing ordinary creation populates asynchronously while holding
                // the lifecycle lock. Wait for that phase before exercising fork.
                let project = project::instance::project().unwrap();
                tokio::time::timeout(std::time::Duration::from_secs(15), async {
                    loop {
                        if let Ok(lock) =
                            super::registry::lifecycle_lock(Path::new(&project.worktree))
                        {
                            drop(lock);
                            break;
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                    }
                })
                .await
                .unwrap();
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
