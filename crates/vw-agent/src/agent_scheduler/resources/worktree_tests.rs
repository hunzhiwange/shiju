#![cfg(not(target_arch = "wasm32"))]

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Result;
use vw_api_types::id::TaskId;
use vw_api_types::task::{TaskPoolWorktreeSlotDto, TaskPoolWorktreeSlotStateDto};

use super::worktree::WorktreeResourceProvider;
use super::{AcquireResourceRequest, ResourceLease, ResourceProvider};
use crate::agent_scheduler::domain::{HolderId, ResourceId};
use crate::agent_scheduler::store::resources::ResourceKind;
use crate::app::agent::project;
use crate::task_worktree_pool::TaskWorktreePoolService;

#[tokio::test]
async fn agent_worktree_provider_maps_pool_slot_to_resource_lease() -> Result<()> {
    let fixture = GitPoolFixture::new();
    fixture.add_slot("slot-ready");

    let pool = TaskWorktreePoolService::with_pool_root_for_tests(fixture.pool_root.clone());
    let provider = WorktreeResourceProvider::new(pool, fixture.project());
    let request = AcquireResourceRequest {
        holder_id: HolderId::new("run-1")?,
        task_id: TaskId("task-alpha".to_string()),
    };

    let first = provider.acquire(request.clone()).await?.expect("worktree lease");
    let metadata = first.metadata_json.as_object().expect("metadata object");
    let expected_path =
        std::fs::canonicalize(fixture.pool_root.join("slot-ready"))?.to_string_lossy().to_string();
    assert_eq!(metadata.get("slot_id").and_then(|value| value.as_str()), Some("slot-ready"));
    assert_eq!(metadata.get("branch").and_then(|value| value.as_str()), Some("vw/task/task-alpha"));
    assert_eq!(metadata.get("path").and_then(|value| value.as_str()), Some(expected_path.as_str()));

    let second = provider.acquire(request.clone()).await?.expect("reused worktree lease");
    assert_eq!(second.resource_id, first.resource_id);
    assert_eq!(second.metadata_json, first.metadata_json);

    provider.release(&first).await?;
    let snapshot = provider.pool().snapshot(&fixture.project()).await?;
    let released = snapshot.slots.iter().find(|slot| slot.id == "slot-ready").expect("slot");
    assert_eq!(released.leased_task_id, None);
    assert!(released.taint_reason.is_none());

    let reacquired = provider.acquire(request).await?.expect("reacquired worktree lease");
    provider.recycle(&reacquired, Some("execution failed".to_string())).await?;
    let snapshot = provider.pool().snapshot(&fixture.project()).await?;
    let recycled = snapshot.slots.iter().find(|slot| slot.id == "slot-ready").expect("slot");
    assert_eq!(recycled.leased_task_id, None);
    assert_eq!(recycled.taint_reason.as_deref(), Some("execution failed"));

    Ok(())
}

#[tokio::test]
async fn agent_worktree_provider_rejects_wrong_kind_and_inactive_leases() -> Result<()> {
    let fixture = GitPoolFixture::new();
    let pool = TaskWorktreePoolService::with_pool_root_for_tests(fixture.pool_root.clone());
    let provider = WorktreeResourceProvider::new(pool, fixture.project());
    let task_id = TaskId("task-no-slot".to_string());
    let holder_id = HolderId::new("run-no-slot")?;

    let mut lease = ResourceLease {
        resource_kind: ResourceKind::WorkerSlot,
        resource_id: ResourceId::new("slot-not-worktree")?,
        holder_id,
        task_id,
        metadata_json: serde_json::Value::Null,
    };
    let release_error = provider.release(&lease).await.unwrap_err().to_string();
    assert!(release_error.contains("not a worktree lease"));
    let recycle_error = provider.recycle(&lease, None).await.unwrap_err().to_string();
    assert!(recycle_error.contains("not a worktree lease"));

    lease.resource_kind = ResourceKind::Worktree;
    let release_error = provider.release(&lease).await.unwrap_err().to_string();
    assert!(release_error.contains("lease is not active for task"));
    let recycle_error = provider.recycle(&lease, None).await.unwrap_err().to_string();
    assert!(recycle_error.contains("lease is not active for task"));

    Ok(())
}

#[test]
fn agent_worktree_provider_rejects_slot_without_leased_task() -> Result<()> {
    let request = acquire_request("run-missing-task", "task-missing-task")?;
    let error = super::worktree::lease_from_slot(request, slot("slot-missing-task", None))
        .unwrap_err()
        .to_string();

    assert!(error.contains("returned a slot without a leased task"));
    Ok(())
}

#[test]
fn agent_worktree_provider_rejects_slot_leased_to_different_task() -> Result<()> {
    let request = acquire_request("run-task-mismatch", "task-requested")?;
    let error = super::worktree::lease_from_slot(
        request,
        slot("slot-task-mismatch", Some(TaskId("task-other".to_string()))),
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("leased task mismatch"));
    assert!(error.contains("task-requested"));
    assert!(error.contains("task-other"));
    Ok(())
}

#[test]
fn agent_worktree_provider_rejects_slot_with_empty_resource_id() -> Result<()> {
    let request = acquire_request("run-empty-slot", "task-empty-slot")?;
    let error = super::worktree::lease_from_slot(
        request,
        slot("   ", Some(TaskId("task-empty-slot".to_string()))),
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("resource id must not be empty"));
    Ok(())
}

#[derive(Debug)]
struct GitPoolFixture {
    _temp: tempfile::TempDir,
    repo: PathBuf,
    pool_root: PathBuf,
}

impl GitPoolFixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().expect("temp dir");
        let repo = temp.path().join("repo");
        let pool_root = temp.path().join("task-worktrees");
        std::fs::create_dir_all(&pool_root).expect("pool root");
        init_repo(&repo);
        Self { _temp: temp, repo, pool_root }
    }

    fn project(&self) -> project::Info {
        project::Info {
            id: "project-1".to_string(),
            worktree: self.repo.to_string_lossy().to_string(),
            vcs: Some(project::Vcs::Git),
            name: None,
            icon: None,
            commands: None,
            time: project::TimeInfo { created: 0, updated: 0, initialized: None },
            sandboxes: Vec::new(),
        }
    }

    fn add_slot(&self, slot_id: &str) {
        let path = self.pool_root.join(slot_id);
        run(Command::new("git")
            .arg("-C")
            .arg(&self.repo)
            .arg("worktree")
            .arg("add")
            .arg("-b")
            .arg(format!("vw/task/{slot_id}"))
            .arg(&path)
            .arg("main"));
        std::fs::write(path.join(format!("{slot_id}.txt")), slot_id).expect("write marker");
    }
}

fn acquire_request(holder_id: &str, task_id: &str) -> Result<AcquireResourceRequest> {
    Ok(AcquireResourceRequest {
        holder_id: HolderId::new(holder_id)?,
        task_id: TaskId(task_id.to_string()),
    })
}

fn slot(id: &str, leased_task_id: Option<TaskId>) -> TaskPoolWorktreeSlotDto {
    TaskPoolWorktreeSlotDto {
        id: id.to_string(),
        path: format!("/tmp/{id}"),
        branch: format!("vw/task/{id}"),
        state: TaskPoolWorktreeSlotStateDto::Busy,
        leased_task_id,
        taint_reason: None,
    }
}

fn init_repo(path: &Path) {
    run(Command::new("git").arg("init").arg("-b").arg("main").arg(path));
    run(Command::new("git")
        .arg("-C")
        .arg(path)
        .arg("config")
        .arg("user.email")
        .arg("test@example.com"));
    run(Command::new("git").arg("-C").arg(path).arg("config").arg("user.name").arg("Test User"));
    std::fs::write(path.join("README.md"), "demo").expect("write readme");
    run(Command::new("git").arg("-C").arg(path).arg("add").arg("README.md"));
    run(Command::new("git").arg("-C").arg(path).arg("commit").arg("-m").arg("init"));
}

fn run(command: &mut Command) {
    let output = command.output().expect("run command");
    assert!(
        output.status.success(),
        "command failed: {:?}\nstdout:\n{}\nstderr:\n{}",
        command,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
