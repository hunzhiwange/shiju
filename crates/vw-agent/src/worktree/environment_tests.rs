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

#[tokio::test]
async fn worktree_environment_runs_in_checkout_before_ready() {
    let (_root, checkout) = fixture();
    let dir = checkout.as_path();
    let info = project(dir, "echo setup > setup.txt");
    environment::prepare(
        dir.to_str().unwrap(),
        &info.id,
        "test -f setup.txt && echo ready > extra.txt",
    )
    .await
    .unwrap();
    assert!(dir.join("extra.txt").exists());
    project::remove_local_test_project(&info.id).unwrap();
}

#[tokio::test]
async fn worktree_environment_failure_prevents_ready() {
    let (_root, checkout) = fixture();
    let dir = checkout.as_path();
    let info = project(dir, "exit 23");
    let result =
        environment::prepare(dir.to_str().unwrap(), &info.id, "echo unsafe > extra.txt").await;
    project::remove_local_test_project(&info.id).unwrap();
    assert!(result.is_err(), "initialization failure must propagate");
    assert!(!dir.join("extra.txt").exists());
}

#[tokio::test]
async fn worktree_environment_retry_does_not_rerun_successful_setup() {
    let (_root, checkout) = fixture();
    let dir = checkout.as_path();
    let info = project(dir, "echo setup >> setup.txt");
    let directory = dir.to_str().unwrap();
    assert!(environment::prepare(directory, &info.id, "test -f allow.txt").await.is_err());
    assert!(environment::ensure_ready(directory).is_err());
    std::fs::write(dir.join("user-edit.txt"), "preserve").unwrap();
    std::fs::write(dir.join("allow.txt"), "retry").unwrap();
    assert!(environment::prepare(directory, &info.id, "").await.is_err());
    environment::retry(directory, &info.id).await.unwrap();
    environment::retry(directory, &info.id).await.unwrap();
    environment::prepare(directory, &info.id, "echo should-not-run > duplicate.txt").await.unwrap();
    environment::ensure_ready(directory).unwrap();
    assert!(!dir.join("duplicate.txt").exists());
    assert_eq!(std::fs::read_to_string(dir.join("user-edit.txt")).unwrap(), "preserve");
    project::remove_local_test_project(&info.id).unwrap();
    assert_eq!(std::fs::read_to_string(dir.join("setup.txt")).unwrap(), "setup\n");
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
async fn worktree_environment_timeout_kills_descendants() {
    let (_root, checkout) = fixture();
    let error = environment::run_command(
        checkout.to_str().unwrap(),
        "sleep 1; echo leaked > late",
        std::time::Duration::from_millis(100),
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("timed out"));
    tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
    assert!(!checkout.join("late").exists());
}

#[tokio::test]
async fn worktree_environment_cancellation_records_failure_and_excludes_duplicates() {
    let (_root, checkout) = fixture();
    let info = project(&checkout, "echo started > started; sleep 1; echo leaked > late");
    let directory = checkout.to_string_lossy().to_string();
    let project_id = info.id.clone();
    let interrupted = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let events = interrupted.clone();
    let id = info.id.clone();
    let unsubscribe = crate::bus::subscribe(super::event::FAILED, move |event| {
        if event["properties"]["projectId"] == id {
            events.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
    });
    let attempt =
        tokio::spawn(async move { environment::prepare(&directory, &project_id, "").await });
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while !checkout.join("started").exists() {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert!(
        environment::prepare(checkout.to_str().unwrap(), &info.id, "")
            .await
            .unwrap_err()
            .to_string()
            .contains("busy")
    );
    attempt.abort();
    assert!(attempt.await.unwrap_err().is_cancelled());
    unsubscribe();
    assert_eq!(interrupted.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert_eq!(
        environment::status(checkout.to_str().unwrap()).unwrap(),
        Some(environment::Status::Error)
    );
    assert!(environment::ensure_ready(checkout.to_str().unwrap()).is_err());
    let mut changed = info.clone();
    changed.commands = None;
    project::store_local_test_project(&changed).unwrap();
    environment::retry(checkout.to_str().unwrap(), &info.id).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
    assert!(!checkout.join("late").exists());
    project::remove_local_test_project(&info.id).unwrap();
}

#[tokio::test]
async fn worktree_environment_extra_failure_is_reported() {
    let (_root, checkout) = fixture();
    let info = project(&checkout, "");
    assert!(
        environment::prepare(checkout.to_str().unwrap(), &info.id, "exit 17")
            .await
            .unwrap_err()
            .to_string()
            .contains("17")
    );
    assert!(environment::ensure_ready(checkout.to_str().unwrap()).is_err());
    project::remove_local_test_project(&info.id).unwrap();
}

#[tokio::test]
async fn worktree_environment_creation_and_fork_publish_ready_after_setup() {
    use crate::project::instance;
    use std::sync::{Arc, Mutex};
    let (root, _checkout) = fixture();
    let info = project(root.path(), "test -f tracked && echo setup >> setup.txt");
    let tenant = isolated_tenant(&info, root.path()).await;
    let context = instance::Context {
        directory: root.path().to_string_lossy().into(),
        worktree: root.path().to_string_lossy().into(),
        project: info.clone(),
    };
    let observed = Arc::new(Mutex::new(vec![]));
    let events = observed.clone();
    let id = info.id.clone();
    let unsubscribe = crate::bus::global_subscribe(move |event| {
        if event.payload["type"] == "worktree.ready"
            && event.payload["properties"]["projectId"] == id
        {
            let directory = std::path::PathBuf::from(event.directory.unwrap());
            events.lock().unwrap().push(directory.join("setup.txt").exists());
        }
    });
    crate::session::ui_store::with_tenant_scope(
        tenant,
        instance::provide_context(context, async {
            let normal = super::create(Some(super::CreateInput {
                name: Some(format!("environment-{}", uuid::Uuid::new_v4())),
                start_command: Some("test -f setup.txt && echo extra > extra.txt".into()),
            }))
            .await
            .unwrap();
            assert!(std::path::Path::new(&normal.directory).join("extra.txt").exists());
            environment::ensure_ready(&normal.directory).unwrap();
            // New chats in an existing successful environment cannot repeat setup.
            environment::prepare(&normal.directory, &info.id, "exit 99").await.unwrap();
            assert_eq!(
                std::fs::read_to_string(std::path::Path::new(&normal.directory).join("setup.txt"))
                    .unwrap(),
                "setup\n"
            );
            super::remove_fork_checkout(normal.directory).await.unwrap();
            let fork = super::create_fork_checkout(Some(&format!(
                "environment-fork-{}",
                uuid::Uuid::new_v4()
            )))
            .await
            .unwrap();
            environment::ensure_ready(&fork.directory).unwrap();
            assert!(std::path::Path::new(&fork.directory).join("setup.txt").exists());
            super::remove_fork_checkout(fork.directory).await.unwrap();
        }),
    )
    .await;
    unsubscribe();
    assert_eq!(*observed.lock().unwrap(), vec![true, true]);
    project::remove_local_test_project(&info.id).unwrap();
}

#[tokio::test]
async fn worktree_environment_creation_failure_can_retry_without_resetting_checkout() {
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

#[tokio::test]
async fn worktree_environment_pending_cannot_be_retried_or_opened() {
    let (_root, checkout) = fixture();
    let info = project(&checkout, "echo unexpected > setup.txt");
    let directory = checkout.to_str().unwrap();
    environment::pending(directory).unwrap();
    assert!(environment::ensure_ready(directory).is_err());
    assert!(environment::retry(directory, &info.id).await.is_err());
    std::fs::create_dir(checkout.join("nested")).unwrap();
    assert!(environment::ensure_ready(checkout.join("nested").to_str().unwrap()).is_err());
    assert!(!checkout.join("setup.txt").exists());
    project::remove_local_test_project(&info.id).unwrap();
}

async fn isolated_tenant(
    info: &project::Info,
    root: &std::path::Path,
) -> vw_api_types::id::TenantScope {
    let tenant =
        vw_api_types::id::TenantScope::new("environment", uuid::Uuid::new_v4().to_string())
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
