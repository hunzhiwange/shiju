#![cfg(unix)]

use super::environment;
use crate::project;

fn project(directory: &std::path::Path, start: &str) -> project::Info {
    let info = project::Info {
        id: format!("environment-{}", uuid::Uuid::new_v4()),
        worktree: directory.to_string_lossy().into(),
        vcs: Some(project::Vcs::Git),
        name: None,
        icon: None,
        commands: Some(project::Commands { start: Some(start.into()) }),
        time: project::TimeInfo { created: 0, updated: 0, initialized: None },
        sandboxes: vec![],
    };
    project::store_local_test_project(&info).unwrap();
    info
}




fn git(path: &std::path::Path, args: &[&str]) {
    let result = std::process::Command::new("git").arg("-C").arg(path).args(args).output().unwrap();
    assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
}
fn fixture() -> (tempfile::TempDir, std::path::PathBuf) {
    let root = tempfile::tempdir().unwrap();
    git(root.path(), &["init", "-b", "main"]);
    std::fs::write(root.path().join("tracked"), "checkout").unwrap();
    git(root.path(), &["add", "tracked"]);
    git(
        root.path(),
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.com",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-m",
            "initial",
        ],
    );
    let checkout = root.path().join("checkout");
    git(root.path(), &["worktree", "add", "-b", "test-environment", checkout.to_str().unwrap()]);
    (root, checkout)
}





#[tokio::test]
async fn worktree_environment_creation_failure_can_retry_without_resetting_checkout() {
    super::initialize_host();
    use crate::project::instance;
    use std::sync::{Arc, Mutex};
    let (root, _checkout) = fixture();
    let info = project(root.path(), "test -f allow.txt && echo setup >> setup.txt");
    let tenant = isolated_tenant(&info, root.path()).await;
    let context = instance::Context {
        directory: root.path().to_string_lossy().into(),
        worktree: root.path().to_string_lossy().into(),
        project: info.clone(),
    };
    let failures = Arc::new(Mutex::new(vec![]));
    let observed = failures.clone();
    let id = info.id.clone();
    let unsubscribe = crate::bus::global_subscribe(move |event| {
        if event.payload["properties"]["projectId"] == id
            && ["worktree.failed", "worktree.ready"]
                .contains(&event.payload["type"].as_str().unwrap_or_default())
        {
            observed.lock().unwrap().push((
                event.payload["type"].as_str().unwrap().to_owned(),
                event.directory.unwrap(),
            ));
        }
    });
    crate::session::ui_store::with_tenant_scope(
        tenant,
        instance::provide_context(context, async {
            assert!(
                super::create(Some(super::CreateInput {
                    name: Some(format!("environment-failure-{}", uuid::Uuid::new_v4())),
                    start_command: None,
                }))
                .await
                .is_err()
            );
            let directory = failures.lock().unwrap()[0].1.clone();
            assert!(environment::ensure_ready(&directory).is_err());
            assert_eq!(failures.lock().unwrap().len(), 1);
            assert_eq!(failures.lock().unwrap()[0].0, "worktree.failed");
            // Public creation must reject a failed environment before allocating a session.
            let result =
                crate::session::session::create_next(crate::session::session::CreateInput {
                    directory: directory.clone(),
                    ..Default::default()
                })
                .await;
            assert!(result.is_err());
            let path = std::path::Path::new(&directory);
            std::fs::write(path.join("tracked"), "user edit").unwrap();
            std::fs::write(path.join("allow.txt"), "retry").unwrap();
            super::retry_environment(directory.clone()).await.unwrap();
            super::retry_environment(directory.clone()).await.unwrap();
            environment::ensure_ready(&directory).unwrap();
            assert_eq!(std::fs::read_to_string(path.join("tracked")).unwrap(), "user edit");
            assert_eq!(std::fs::read_to_string(path.join("setup.txt")).unwrap(), "setup\n");
            super::remove_fork_checkout(directory).await.unwrap();
        }),
    )
    .await;
    unsubscribe();
    project::remove_local_test_project(&info.id).unwrap();
}


async fn isolated_tenant(
    info: &project::Info,
    root: &std::path::Path,
) -> vw_api_transport::TenantScope {
    let tenant =
        vw_api_transport::TenantScope::new("environment", uuid::Uuid::new_v4().to_string())
            .unwrap();
    project::runtime_store().unwrap().create(&tenant, info).unwrap();
    crate::config::update_global_for_tenant(
        &tenant,
        serde_json::json!({"worktree_settings": {"root": root.join("managed")}}),
    )
    .await
    .unwrap();
    tenant
}
