use super::{
    cleanup::*,
    registry::Record,
};
use {
    vw_api_transport::id::ProjectId,
    vw_api_transport::id::ProjectScope,
    vw_api_transport::id::TenantScope,
};

fn scope(project: &str) -> ProjectScope {
    ProjectScope::for_default_tenant(ProjectId::from(project)).unwrap()
}

use super::registry;
use crate::session::{session::Info, status, store::SessionStore};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    process::Command,
};

fn git(path: &Path, args: &[&str]) {
    let output = Command::new("git").arg("-C").arg(path).args(args).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}
struct Fixture {
    _temp: tempfile::TempDir,
    primary: PathBuf,
    scope: ProjectScope,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let primary = temp.path().join("primary");
        std::fs::create_dir(&primary).unwrap();
        git(&primary, &["init", "-b", "main"]);
        git(
            &primary,
            &[
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.com",
                "commit",
                "--allow-empty",
                "-m",
                "init",
            ],
        );
        Self { _temp: temp, primary, scope: scope(&uuid::Uuid::new_v4().to_string()) }
    }
    async fn checkout(&self, name: &str) -> Record {
        self.checkout_for(name, &self.scope).await
    }
    async fn checkout_for(&self, name: &str, scope: &ProjectScope) -> Record {
        let directory = self._temp.path().join(name);
        git(&self.primary, &["worktree", "add", "-b", name, directory.to_str().unwrap()]);
        registry::register(scope, &self.primary, &directory).await.unwrap()
    }
    fn snapshot(&self) -> ProtectionSnapshot {
        ProtectionSnapshot {
            sessions: vec![],
            foreign_directories: vec![],
            pinned: HashMap::new(),
            busy: HashSet::new(),
        }
    }
}
fn session(record: &Record, id: &str, title: &str, updated: u64) -> Info {
    serde_json::from_value(serde_json::json!({
        "id": id, "slug": id, "projectID": record.scope.project_id().as_ref(),
        "directory": record.directory, "title": title, "version": "test",
        "time": { "created": 1, "updated": updated }
    }))
    .unwrap()
}
#[tokio::test]
async fn worktree_cleanup_all_associated_sessions_protect_and_latest_title_wins() {
    super::initialize_host();
    let f = Fixture::new();
    let mut first = f.checkout("first").await;
    let mut second = f.checkout("second").await;
    first.last_used_at_ms = 1;
    second.last_used_at_ms = 2;
    let old = session(&first, "old", "Old pinned", 10);
    let mut latest = session(&first, "new", "Recent chat", 30);
    // A session scoped to another project can use this same physical checkout.
    latest.project_id = "other-project".into();
    let mut snapshot = f.snapshot();
    snapshot.sessions = vec![latest.clone(), old.clone()];
    snapshot.pinned.insert(old.project_id.clone(), HashSet::from([old.id.clone()]));
    snapshot.pinned.insert(latest.project_id.clone(), HashSet::new());
    let first_observed = observe(&f.primary, first.clone(), Some(&snapshot));
    assert_eq!(first_observed.reasons, [ProtectionReason::Pinned]);
    assert_eq!(first_observed.title, "Recent chat");
    assert_eq!(first_observed.last_used_at_ms, 30);
    let p = plan(
        &f.scope,
        &settings(1),
        vec![first_observed, observe(&f.primary, second.clone(), Some(&snapshot))],
    );
    assert_eq!(p.candidates[0].id, second.id);
    snapshot.pinned.get_mut(&old.project_id).unwrap().clear();
    snapshot.busy = super::super::host_adapter::busy_sessions(HashMap::from([(old.id.clone(), status::Info::Busy)]));
    assert_eq!(
        observe(&f.primary, first.clone(), Some(&snapshot)).reasons,
        [ProtectionReason::Busy]
    );
    snapshot.busy = super::super::host_adapter::busy_sessions(HashMap::from([(old.id.clone(), status::Info::Retry { attempt: 1, message: "retry".into(), next: 100 })]));
    assert_eq!(
        observe(&f.primary, first.clone(), Some(&snapshot)).reasons,
        [ProtectionReason::Busy]
    );
    snapshot.busy.clear();
    snapshot.sessions[1].mark_first_response_pending();
    assert_eq!(
        observe(&f.primary, first.clone(), Some(&snapshot)).reasons,
        [ProtectionReason::Busy]
    );
    snapshot.sessions[1].clear_first_response_pending();
    let p = plan(
        &f.scope,
        &settings(1),
        vec![
            observe(&f.primary, first, Some(&snapshot)),
            observe(&f.primary, second.clone(), Some(&snapshot)),
        ],
    );
    assert_eq!(p.candidates[0].id, second.id); // actual last chat use outranks registration
}

#[tokio::test]
async fn worktree_cleanup_session_store_preserves_pending_duplicates_and_errors() {
    super::initialize_host();
    let f = Fixture::new();
    let record = f.checkout("owned").await;
    let db = crate::global_database::GlobalDatabase::new(f._temp.path().join("sessions.sqlite3"))
        .unwrap();
    let projects = crate::project::store::ProjectStore::new(db.clone()).unwrap();
    let add_project = |scope: &ProjectScope| {
        projects
            .create(
                scope.tenant_scope(),
                &project_types::Info {
                    id: scope.project_id().as_ref().to_owned(),
                    worktree: f.primary.to_string_lossy().into_owned(),
                    vcs: None,
                    name: None,
                    icon: None,
                    commands: None,
                    time: project_types::TimeInfo {
                        created: 1,
                        updated: 1,
                        initialized: None,
                    },
                    sandboxes: vec![],
                },
            )
            .unwrap();
    };
    add_project(&f.scope);
    let store = SessionStore::new(db.clone()).unwrap();
    let first = session(&record, "same-id", "first", 10);
    let mut second = first.clone();
    second.project_id = "other-project".into();
    second.mark_first_response_pending();
    let other_project =
        ProjectScope::new(f.scope.tenant_scope().clone(), ProjectId::from("other-project"))
            .unwrap();
    store.save_agent_session(&f.scope, &first).unwrap();
    add_project(&other_project);
    store.reserve_pending_session(&other_project, &second).unwrap();
    let loaded = store.load_agent_deletion_candidates(f.scope.tenant_scope()).unwrap();
    assert_eq!(loaded.len(), 2);
    assert!(loaded.iter().any(Info::is_first_response_pending));
    let other = ProjectScope::new(
        TenantScope::new("other", "company").unwrap(),
        f.scope.project_id().clone(),
    )
    .unwrap();
    add_project(&other);
    let primary = f.primary.to_string_lossy().into_owned();
    assert_eq!(
        store.foreign_worktree_directories(f.scope.tenant_scope()).unwrap(),
        [primary.clone()]
    );
    store.save_agent_session(&other, &first).unwrap();
    let directories = store.foreign_worktree_directories(f.scope.tenant_scope()).unwrap();
    assert_eq!(directories.len(), 2);
    assert!(directories.contains(&primary));
    assert!(directories.contains(&first.directory));
    db.connect_sqlite()
        .unwrap()
        .execute(
            "UPDATE session_agent_sessions SET info_json = 'broken' WHERE platform_id = ?1",
            [other.platform_id().as_ref()],
        )
        .unwrap();
    assert!(store.foreign_worktree_directories(f.scope.tenant_scope()).is_err());
    db.connect_sqlite()
        .unwrap()
        .execute("UPDATE session_agent_sessions SET info_json = 'broken'", [])
        .unwrap();
    assert!(store.load_agent_deletion_candidates(f.scope.tenant_scope()).is_err());
}

// Allow temporary OS lock contention while retaining the identity/error assertions.
