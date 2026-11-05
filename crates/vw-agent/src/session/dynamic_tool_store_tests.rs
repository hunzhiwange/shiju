use super::dynamic_tool_store::PersistentToolPolicy;
use super::store::SessionStore;
use crate::global_database::GlobalDatabase;
use crate::project::store::ProjectStore;
use crate::tools::catalog::ToolCatalog;
use crate::tools::{Tool, ToolResult};
use anyhow::Result;
use serde_json::{Value, json};
use std::sync::Arc;
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};
use vw_shared::project::{Info, TimeInfo};
use vw_shared::session::ui_types::ChatSession;

struct VersionTool(&'static str);
#[async_trait::async_trait]
impl Tool for VersionTool {
    fn name(&self) -> &str {
        "persistent_tool"
    }
    fn description(&self) -> &str {
        self.0
    }
    fn parameters_schema(&self) -> Value {
        json!({"type":"object"})
    }
    async fn execute(&self, _: Value) -> Result<ToolResult> {
        Ok(ToolResult { success: true, output: self.0.into(), error: None })
    }
}
fn tools(value: &'static str) -> Vec<Arc<dyn Tool>> {
    vec![Arc::new(VersionTool(value))]
}

pub(crate) fn fixture(
    database: GlobalDatabase,
    tenant: &str,
) -> Result<(SessionStore, ProjectScope)> {
    fixture_project(database, tenant, "project")
}

fn fixture_project(
    database: GlobalDatabase,
    tenant: &str,
    project: &str,
) -> Result<(SessionStore, ProjectScope)> {
    let projects = ProjectStore::new(database.clone())?;
    let scope = ProjectScope::new(TenantScope::new("platform", tenant)?, ProjectId::from(project))?;
    projects.create(
        scope.tenant_scope(),
        &Info {
            id: project.into(),
            worktree: format!("/work/{tenant}/{project}"),
            vcs: None,
            name: None,
            icon: None,
            commands: None,
            time: TimeInfo { created: 1, updated: 1, initialized: None },
            sandboxes: vec![],
        },
    )?;
    let store = SessionStore::new(database)?;
    for id in ["session", "other"] {
        store.save_session(
            &scope,
            &ChatSession {
                id: id.into(),
                title: "session".into(),
                messages: vec![],
                message_ids: vec![],
                calls: vec![],
                steps: vec![],
                created_ms: 1,
                updated_ms: 1,
            },
        )?;
    }
    Ok((store, scope))
}

pub(crate) fn failed_write(database: GlobalDatabase) -> Result<()> {
    let (store, scope) = fixture(database.clone(), "failure")?;
    let catalog = ToolCatalog::default();
    let policy = PersistentToolPolicy::trusted_host();
    let old = store.replace_dynamic_tools(
        &scope,
        "session",
        &catalog,
        tools("old"),
        ["persistent_tool".into()],
        policy,
    )?;
    match database.provider() {
        crate::global_database::config::DatabaseProvider::Sqlite => database.connect_sqlite()?.execute_batch(
            "CREATE TRIGGER reject_dynamic BEFORE UPDATE ON session_dynamic_tools BEGIN SELECT RAISE(ABORT, 'injected failure'); END;"
        )?,
        crate::global_database::config::DatabaseProvider::Postgres => database.connect_postgres()?.batch_execute(
            "CREATE FUNCTION reject_dynamic() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected failure'; END $$;
             CREATE TRIGGER reject_dynamic BEFORE UPDATE ON session_dynamic_tools FOR EACH ROW EXECUTE FUNCTION reject_dynamic();"
        )?,
    }
    assert!(
        store
            .replace_dynamic_tools(
                &scope,
                "session",
                &catalog,
                tools("new"),
                ["persistent_tool".into()],
                policy
            )
            .is_err()
    );
    assert!(Arc::ptr_eq(&old, &catalog.snapshot()));
    let stored = store.load_dynamic_tools(&scope, "session")?.unwrap();
    assert_eq!(stored.version, old.version());
    assert_eq!(stored.specs[0].description, "old");
    Ok(())
}

#[test]
fn dynamic_tools_persistence_failed_write_keeps_running_catalog() -> Result<()> {
    let home = tempfile::tempdir()?;
    failed_write(GlobalDatabase::new(home.path().join("db.sqlite3"))?)
}

pub(crate) fn success_and_empty(database: GlobalDatabase) -> Result<()> {
    let (store, scope) = fixture(database, "success")?;
    let catalog = ToolCatalog::default();
    assert!(store.load_dynamic_tools(&scope, "session")?.is_none());
    let first = store.replace_dynamic_tools(
        &scope,
        "session",
        &catalog,
        tools("first"),
        ["persistent_tool".into()],
        PersistentToolPolicy::trusted_host(),
    )?;
    let record = store.load_dynamic_tools(&scope, "session")?.unwrap();
    assert_eq!(record.version, first.version());
    assert_eq!(record.specs[0].description, "first");
    let empty = store.replace_dynamic_tools(
        &scope,
        "session",
        &catalog,
        vec![],
        [],
        PersistentToolPolicy::trusted_host(),
    )?;
    assert!(empty.version() > first.version());
    assert!(catalog.snapshot().specs().is_empty());
    assert_eq!(first.specs()[0].description, "first");
    let record = store.load_dynamic_tools(&scope, "session")?.unwrap();
    assert!(record.specs.is_empty());
    assert_eq!(record.version, empty.version());
    store.delete_session(&scope, "session")?;
    assert!(store.load_dynamic_tools(&scope, "session")?.is_none());
    Ok(())
}

pub(crate) fn rejects_untrusted_and_invalid(database: GlobalDatabase) -> Result<()> {
    let (store, scope) = fixture(database, "denied")?;
    let catalog = ToolCatalog::default();
    let initial = catalog.snapshot();
    let error = store
        .replace_dynamic_tools(
            &scope,
            "session",
            &catalog,
            tools("denied"),
            ["persistent_tool".into()],
            PersistentToolPolicy::default(),
        )
        .unwrap_err();
    assert!(error.to_string().contains("trusted host"));
    assert!(
        store
            .replace_dynamic_tools(
                &scope,
                "session",
                &catalog,
                tools("bad grant"),
                ["unregistered".into()],
                PersistentToolPolicy::trusted_host()
            )
            .is_err()
    );
    assert!(
        store
            .replace_dynamic_tools(
                &scope,
                "session",
                &catalog,
                vec![Arc::new(VersionTool("duplicate")), Arc::new(VersionTool("duplicate"))],
                ["persistent_tool".into()],
                PersistentToolPolicy::trusted_host()
            )
            .is_err()
    );
    assert!(Arc::ptr_eq(&initial, &catalog.snapshot()));
    assert!(store.load_dynamic_tools(&scope, "session")?.is_none());
    Ok(())
}

pub(crate) fn isolation_and_stale(database: GlobalDatabase) -> Result<()> {
    let (store, scope) = fixture(database.clone(), "tenant-a")?;
    let (_, other_scope) = fixture(database.clone(), "tenant-b")?;
    let (_, other_project) = fixture_project(database, "tenant-a", "other-project")?;
    let catalog = ToolCatalog::default();
    store.replace_dynamic_tools(
        &scope,
        "session",
        &catalog,
        tools("a"),
        ["persistent_tool".into()],
        PersistentToolPolicy::trusted_host(),
    )?;
    assert!(store.load_dynamic_tools(&scope, "other")?.is_none());
    assert!(store.load_dynamic_tools(&other_scope, "session")?.is_none());
    assert!(store.load_dynamic_tools(&other_project, "session")?.is_none());
    let stale = ToolCatalog::default();
    assert!(
        store
            .replace_dynamic_tools(
                &scope,
                "session",
                &stale,
                tools("stale"),
                ["persistent_tool".into()],
                PersistentToolPolicy::trusted_host()
            )
            .is_err()
    );
    assert_eq!(stale.snapshot().version(), 0);
    assert_eq!(store.load_dynamic_tools(&scope, "session")?.unwrap().specs[0].description, "a");
    assert!(
        store
            .replace_dynamic_tools(
                &scope,
                "missing",
                &catalog,
                vec![],
                [],
                PersistentToolPolicy::trusted_host()
            )
            .is_err()
    );
    assert_eq!(catalog.snapshot().version(), 1);
    Ok(())
}

pub(crate) fn concurrent(database: GlobalDatabase) -> Result<()> {
    let (store, scope) = fixture(database, "concurrent")?;
    let catalog = Arc::new(ToolCatalog::default());
    let barrier = Arc::new(std::sync::Barrier::new(3));
    std::thread::scope(|threads| -> Result<()> {
        let mut handles = vec![];
        for value in ["one", "two"] {
            let (store, scope, catalog, barrier) =
                (&store, &scope, catalog.clone(), barrier.clone());
            handles.push(threads.spawn(move || {
                barrier.wait();
                store.replace_dynamic_tools(
                    scope,
                    "session",
                    &catalog,
                    tools(value),
                    ["persistent_tool".into()],
                    PersistentToolPolicy::trusted_host(),
                )
            }));
        }
        barrier.wait();
        let mut versions = vec![];
        for handle in handles {
            versions.push(handle.join().unwrap()?.version());
        }
        versions.sort();
        assert_eq!(versions, vec![1, 2]);
        Ok(())
    })?;
    let record = store.load_dynamic_tools(&scope, "session")?.unwrap();
    assert_eq!(record.version, catalog.snapshot().version());
    assert_eq!(record.specs[0].description, catalog.snapshot().specs()[0].description);
    Ok(())
}

#[test]
fn dynamic_tools_persistence_success_and_explicit_empty() -> Result<()> {
    let home = tempfile::tempdir()?;
    success_and_empty(GlobalDatabase::new(home.path().join("db.sqlite3"))?)
}
#[test]
fn dynamic_tools_persistence_rejects_untrusted_and_invalid() -> Result<()> {
    let home = tempfile::tempdir()?;
    rejects_untrusted_and_invalid(GlobalDatabase::new(home.path().join("db.sqlite3"))?)
}
#[test]
fn dynamic_tools_persistence_scopes_and_stale_versions_are_isolated() -> Result<()> {
    let home = tempfile::tempdir()?;
    isolation_and_stale(GlobalDatabase::new(home.path().join("db.sqlite3"))?)
}
#[test]
fn dynamic_tools_persistence_concurrent_replacements_follow_commit_order() -> Result<()> {
    let home = tempfile::tempdir()?;
    concurrent(GlobalDatabase::new(home.path().join("db.sqlite3"))?)
}

#[test]
fn dynamic_tools_persistence_serialization_error_keeps_running_catalog() -> Result<()> {
    // The current spec representation is JSON-only and serializes to String infallibly.
    // Exercise the fallible serialization boundary with a deterministic failing sink.
    struct RejectWriter;
    impl std::io::Write for RejectWriter {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("injected serialization sink failure"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let catalog = ToolCatalog::default();
    let old = catalog.replace(tools("old"), ["persistent_tool".into()])?;
    let error = catalog
        .replace_persisted(tools("new"), ["persistent_tool".into()], |snapshot| {
            serde_json::to_writer(RejectWriter, &snapshot.specs())?;
            Ok(())
        })
        .unwrap_err();
    assert!(error.downcast_ref::<serde_json::Error>().unwrap().is_io());
    assert!(Arc::ptr_eq(&old, &catalog.snapshot()));
    Ok(())
}
