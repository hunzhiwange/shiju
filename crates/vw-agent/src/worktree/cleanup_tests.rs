use super::{
    cleanup::*,
    registry::{Lifecycle, Record},
};
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};
use vw_config_types::worktree::WorktreeSettings;

fn scope(project: &str) -> ProjectScope {
    ProjectScope::for_default_tenant(ProjectId::from(project)).unwrap()
}
fn entry(id: &str, used: u64, reasons: Vec<ProtectionReason>) -> Observation {
    Observation {
        record: Record {
            id: id.into(),
            scope: scope("a"),
            directory: format!("/trees/{id}").into(),
            common_dir: "/repo/.git".into(),
            common_identity: "common".into(),
            git_dir: format!("/repo/.git/worktrees/{id}").into(),
            git_identity: id.into(),
            checkout_identity: id.into(),
            created_at_ms: 1,
            last_used_at_ms: used,
            state: Lifecycle::Ready,
            recovered_from: None,
        },
        title: id.into(),
        last_used_at_ms: used,
        reasons,
    }
}
fn settings(keep_count: u32) -> WorktreeSettings {
    WorktreeSettings { keep_count, ..Default::default() }
}
#[test]
fn worktree_cleanup_keeps_pinned_busy_and_pool_worktrees() {
    let p = plan(
        &scope("a"),
        &settings(1),
        vec![
            entry("pin", 1, vec![ProtectionReason::Pinned]),
            entry("busy", 2, vec![ProtectionReason::Busy]),
            entry("slot-x", 3, vec![ProtectionReason::Pool]),
            entry("ready", 4, vec![]),
        ],
    );
    assert_eq!(p.candidates.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(), ["ready"]);
    assert_eq!(p.protected.len(), 3);
    assert_eq!(p.remaining_excess, 1); // pool does not consume ordinary retention
}
#[test]
fn worktree_cleanup_orders_by_last_use() {
    let mut other = entry("foreign-project", 0, vec![]);
    other.record.scope = scope("b");
    let mut tenant = entry("foreign-tenant", 0, vec![]);
    tenant.record.scope =
        ProjectScope::new(TenantScope::new("other", "other").unwrap(), ProjectId::from("a"))
            .unwrap();
    let p = plan(
        &scope("a"),
        &settings(1),
        vec![entry("c", 3, vec![]), entry("b", 1, vec![]), entry("a", 1, vec![]), other, tenant],
    );
    assert_eq!(p.candidates.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(), ["a", "b"]);
    let repeat = plan(
        &scope("a"),
        &settings(1),
        vec![entry("a", 1, vec![]), entry("b", 1, vec![]), entry("c", 3, vec![])],
    );
    assert_eq!(p.candidates[0].fingerprint, repeat.candidates[0].fingerprint);
    let changed = plan(
        &scope("a"),
        &settings(1),
        vec![entry("a", 2, vec![]), entry("b", 1, vec![]), entry("c", 3, vec![])],
    );
    assert_ne!(p.candidates[0].fingerprint, changed.candidates[1].fingerprint);
    let p = plan(
        &scope("a"),
        &settings(15),
        (0..16).map(|i| entry(&format!("{i:02}"), i, vec![])).collect(),
    );
    assert_eq!(p.candidates.len(), 1);
    assert_eq!(p.candidates[0].id, "00");
    let mut disabled = settings(1);
    disabled.auto_cleanup_enabled = false;
    assert!(
        plan(&scope("a"), &disabled, vec![entry("a", 1, vec![]), entry("b", 2, vec![])])
            .candidates
            .is_empty()
    );
}
#[test]
fn worktree_cleanup_fails_closed_when_protection_is_unknown() {
    let p = plan(
        &scope("a"),
        &settings(1),
        vec![
            entry("a", 1, vec![ProtectionReason::Unknown]),
            entry("b", 2, vec![ProtectionReason::SharedRepository]),
        ],
    );
    assert!(p.candidates.is_empty());
    assert_eq!(p.protected.len(), 2);
    assert_eq!(p.remaining_excess, 1);
}

use super::{environment, registry};
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
            statuses: HashMap::new(),
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
    snapshot.statuses.insert(old.id.clone(), status::Info::Busy);
    assert_eq!(
        observe(&f.primary, first.clone(), Some(&snapshot)).reasons,
        [ProtectionReason::Busy]
    );
    snapshot.statuses.insert(
        old.id.clone(),
        status::Info::Retry { attempt: 1, message: "retry".into(), next: 100 },
    );
    assert_eq!(
        observe(&f.primary, first.clone(), Some(&snapshot)).reasons,
        [ProtectionReason::Busy]
    );
    snapshot.statuses.clear();
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
async fn worktree_cleanup_initialization_pool_external_and_unknown_are_protected() {
    let f = Fixture::new();
    let ready = f.checkout("ready").await;
    let pool = f.checkout("slot-test").await;
    let snapshot = f.snapshot();
    environment::pending(ready.directory.to_str().unwrap()).unwrap();
    assert_eq!(
        observe(&f.primary, ready.clone(), Some(&snapshot)).reasons,
        [ProtectionReason::Initializing]
    );
    let state = ready.git_dir.join("shiju-environment.json");
    std::fs::write(
        &state,
        r#"{"status":"Running","project_done":false,"extra_done":false,"extra_command":""}"#,
    )
    .unwrap();
    assert_eq!(
        observe(&f.primary, ready.clone(), Some(&snapshot)).reasons,
        [ProtectionReason::Initializing]
    );
    std::fs::write(&state, b"corrupt").unwrap();
    assert_eq!(
        observe(&f.primary, ready.clone(), Some(&snapshot)).reasons,
        [ProtectionReason::Unknown]
    );
    assert_eq!(
        observe(&f.primary, pool.clone(), Some(&snapshot)).reasons,
        [ProtectionReason::Pool]
    );
    assert_eq!(
        observe(&f.primary, pool.clone(), None).reasons,
        [ProtectionReason::Pool, ProtectionReason::Unknown]
    );
    let mut primary = pool.clone();
    primary.directory = f.primary.clone();
    assert!(
        observe(&f.primary, primary, Some(&snapshot)).reasons.contains(&ProtectionReason::Unknown)
    );
    std::fs::remove_file(pool.git_dir.join("shiju-managed.json")).unwrap();
    assert!(
        observe(&f.primary, pool, Some(&snapshot)).reasons.contains(&ProtectionReason::Unknown)
    );
    let p = propose(&f.scope, &f.primary, &settings(1)).await.unwrap();
    assert!(p.candidates.is_empty());
}

#[tokio::test]
async fn worktree_cleanup_shared_repository_fails_closed_across_tenants() {
    let f = Fixture::new();
    let record = f.checkout("owned").await;
    let other = ProjectScope::new(
        TenantScope::new("foreign", "company").unwrap(),
        ProjectId::from("foreign"),
    )
    .unwrap();
    let foreign = f.checkout_for("foreign", &other).await;
    let mut snapshot = f.snapshot();
    assert_eq!(
        observe(&f.primary, record.clone(), Some(&snapshot)).reasons,
        [ProtectionReason::SharedRepository]
    );
    // A foreign tenant using the primary also protects every checkout.
    std::fs::remove_file(foreign.git_dir.join("shiju-managed.json")).unwrap();
    snapshot.foreign_directories = vec![f.primary.to_string_lossy().into_owned()];
    assert_eq!(
        observe(&f.primary, record.clone(), Some(&snapshot)).reasons,
        [ProtectionReason::SharedRepository]
    );
    let unrelated = Fixture::new();
    snapshot.foreign_directories = vec![unrelated.primary.to_string_lossy().into_owned()];
    assert!(observe(&f.primary, record.clone(), Some(&snapshot)).reasons.is_empty());
    snapshot.foreign_directories.push("/missing/foreign/workspace".into());
    assert_eq!(
        observe(&f.primary, record.clone(), Some(&snapshot)).reasons,
        [ProtectionReason::Unknown]
    );
    snapshot.foreign_directories = vec![".".into()];
    assert_eq!(observe(&f.primary, record, Some(&snapshot)).reasons, [ProtectionReason::Unknown]);
}

#[tokio::test]
async fn worktree_cleanup_session_store_preserves_pending_duplicates_and_errors() {
    let f = Fixture::new();
    let record = f.checkout("owned").await;
    let db = crate::global_database::GlobalDatabase::new(f._temp.path().join("sessions.sqlite3"))
        .unwrap();
    let projects = crate::project::store::ProjectStore::new(db.clone()).unwrap();
    let add_project = |scope: &ProjectScope| {
        projects
            .create(
                scope.tenant_scope(),
                &vw_shared::project::Info {
                    id: scope.project_id().as_ref().to_owned(),
                    worktree: f.primary.to_string_lossy().into_owned(),
                    vcs: None,
                    name: None,
                    icon: None,
                    commands: None,
                    time: vw_shared::project::TimeInfo {
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

#[tokio::test]
async fn worktree_cleanup_record_use_is_monotonic_and_identity_checked() {
    let f = Fixture::new();
    let record = f.checkout("owned").await;
    let used = record.last_used_at_ms + 100;
    record_use_after_contention(&f.scope, &record.directory, used).await.unwrap();
    record_use_after_contention(&f.scope, &record.directory, used - 1).await.unwrap();
    assert_eq!(
        registry::authorize(&f.scope, &f.primary, &record.id).await.unwrap().last_used_at_ms,
        used
    );
    let other = scope("unrelated");
    record_use_after_contention(&other, &record.directory, used + 1).await.unwrap();
    assert_eq!(
        registry::authorize(&f.scope, &f.primary, &record.id).await.unwrap().last_used_at_ms,
        used
    );
    std::fs::rename(&record.directory, f._temp.path().join("original")).unwrap();
    std::fs::create_dir(&record.directory).unwrap();
    assert!(matches!(record_use_after_contention(&f.scope, &record.directory, used + 1).await,
        Err(super::Error::Invalid(message)) if message == "Managed checkout identity changed"));
}

// Allow temporary OS lock contention while retaining the identity/error assertions.
async fn record_use_after_contention(
    scope: &ProjectScope,
    directory: &Path,
    used: u64,
) -> Result<(), super::Error> {
    for attempt in 0..50 {
        let result = registry::record_use(scope, directory, used);
        if !matches!(&result, Err(super::Error::Invalid(message))
            if message == "Managed registry is busy in another operation")
            || attempt == 49
        {
            return result;
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    unreachable!("bounded attempts always return their final result")
}
